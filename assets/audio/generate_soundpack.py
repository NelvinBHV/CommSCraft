#!/usr/bin/env python3
"""Render the three user-approved CommSCraft sounds.

Scores: Pulse and Beat Call V5, Signal Message V4. Requires TimGM6mb,
FluidSynth, NumPy, SciPy, soundfile and ffmpeg. This rebuild is intended to
preserve the arrangements of the approved audio.
"""
from pathlib import Path
import ctypes as C
import hashlib
import subprocess
import numpy as np
import soundfile as sf
from scipy import signal

OUT = Path(__file__).resolve().parent
SOUNDFONT = "/usr/share/sounds/sf2/TimGM6mb.sf2"
RATE = 44100
assert Path(SOUNDFONT).is_file(), "Install timgm6mb-soundfont"
lib = C.CDLL("libfluidsynth.so.3")
ptr = C.c_void_p
def api(name, ret, types):
    fn = getattr(lib, name)
    fn.restype, fn.argtypes = ret, types
    return fn
settings_new = api("new_fluid_settings",ptr,[])
setnum = api("fluid_settings_setnum",C.c_int,[ptr,C.c_char_p,C.c_double])
setint = api("fluid_settings_setint",C.c_int,[ptr,C.c_char_p,C.c_int])
synth_new = api("new_fluid_synth",ptr,[ptr])
sfload = api("fluid_synth_sfload",C.c_int,[ptr,C.c_char_p,C.c_int])
sel = api("fluid_synth_program_select",C.c_int,[ptr,C.c_int,C.c_int,C.c_int,C.c_int])
on = api("fluid_synth_noteon",C.c_int,[ptr,C.c_int,C.c_int,C.c_int])
off = api("fluid_synth_noteoff",C.c_int,[ptr,C.c_int,C.c_int])
cc = api("fluid_synth_cc",C.c_int,[ptr,C.c_int,C.c_int,C.c_int])
write = api("fluid_synth_write_float",C.c_int,[ptr,C.c_int,ptr,C.c_int,C.c_int,ptr,C.c_int,C.c_int])
delete_synth = api("delete_fluid_synth",None,[ptr])
delete_settings = api("delete_fluid_settings",None,[ptr])

class Score:
    def __init__(self, bpm, v4=False):
        self.bpm = bpm
        self.v4 = v4
        self.events = []
    def add(self, beat, channel, pitch, duration, velocity):
        a = beat*60/self.bpm
        b = (beat+duration)*60/self.bpm
        self.events.append((a,1,channel,int(pitch),int(velocity)))
        self.events.append((b,0,channel,int(pitch),0))
    def chord(self, beat, channel, pitches, duration, velocity, spread=0):
        for k,n in enumerate(pitches):
            self.add(beat+k*spread,channel,n,duration-k*spread,velocity-(max(0,k-1)*3 if self.v4 else 0))
    def render(self, length, programs, volumes, pan, drums=False):
        st = settings_new()
        setnum(st,b"synth.sample-rate",RATE)
        setnum(st,b"synth.gain",0.28 if self.v4 else 0.32)
        setnum(st,b"synth.reverb.room-size",0.45 if self.v4 else 0.28)
        setnum(st,b"synth.reverb.level",0.18 if self.v4 else 0.11)
        setint(st,b"synth.reverb.active",1)
        setint(st,b"synth.chorus.active",1 if self.v4 else 0)
        setint(st,b"synth.polyphony",256 if self.v4 else 128)
        if self.v4: setnum(st,b"synth.chorus.level",1.0)
        sy = synth_new(st)
        soundfont_id = sfload(sy,SOUNDFONT.encode(),1)
        if soundfont_id<0: raise RuntimeError("Cannot load soundfont")
        for ch,program in programs.items():
            sel(sy,ch,soundfont_id,0,program)
            cc(sy,ch,7,volumes.get(ch,70))
            cc(sy,ch,10,pan.get(ch,64))
            cc(sy,ch,91,45 if self.v4 else 18)
            if self.v4: cc(sy,ch,93,25)
        if drums:
            cc(sy,9,7,70 if self.v4 else volumes.get(9,60))
            if self.v4:
                cc(sy,9,10,64)
                cc(sy,9,91,20)
        n = round(length*RATE)
        result = np.zeros((n,2),np.float32)
        def chunk(p,q):
            while p<q:
                k=min(2048 if self.v4 else 4096,q-p)
                left=np.empty(k,np.float32); right=np.empty(k,np.float32)
                write(sy,k,left.ctypes.data,0,1,right.ctypes.data,0,1)
                result[p:p+k,0]=left
                result[p:p+k,1]=right
                p+=k
        previous=0
        for moment,typ,ch,note,vel in sorted(self.events,key=lambda x:(x[0],x[1])):
            pos=min(n,max(0, (int(moment*RATE) if self.v4 else round(moment*RATE))))
            if pos>previous: chunk(previous,pos); previous=pos
            if typ: on(sy,ch,note,vel)
            else: off(sy,ch,note)
        if previous<n: chunk(previous,n)
        delete_synth(sy);delete_settings(st)
        return result

def master_v5(a, target, loop):
    hp=signal.butter(2,50,fs=RATE,btype="highpass",output="sos")
    lp=signal.butter(2,9000,fs=RATE,btype="lowpass",output="sos")
    a=signal.sosfilt(hp,a,axis=0)
    a=signal.sosfilt(lp,a,axis=0)
    if loop:
        k=int(.012*RATE)
        last=a[-k:].copy(); start=a[:k].copy()
        seam=(last[-1]+start[0])*.5
        t=np.linspace(0,1,k)[:,None]
        a[-k:]=last*(1-t)+t*seam
        a[:k]=seam*(1-t)+start*t
    else:
        k=int(.012*RATE); a[:k]*=np.linspace(0,1,k)[:,None]
        k=int(.25*RATE); a[-k:]*=np.linspace(1,0,k)[:,None]
    gain=10**(target/20)/(np.sqrt(np.mean(a*a))+1e-9)
    a*=gain
    peak=np.max(np.abs(a))
    if peak>.91: a*=.91/peak
    return a.astype(np.float32)

def master_v4_notification(a):
    a=a.astype(np.float64)
    b,c=signal.butter(2,60/(RATE/2),"highpass")
    a=signal.lfilter(b,c,a,axis=0)
    a=signal.sosfilt(signal.butter(2,7500,fs=RATE,output="sos"),a,axis=0)
    start=min(int(.008*RATE),len(a)//3)
    end=min(int(.45*RATE),len(a)//3)
    a[:start]*=np.linspace(0,1,start)[:,None]
    a[-end:]*=np.linspace(1,0,end)[:,None]
    a*=10**(-21/20)/(np.sqrt(np.mean(a*a))+1e-8)
    peak=np.max(np.abs(a))
    if peak>.93: a*=.93/peak
    return a.astype(np.float32)

def build_hold():
    score=Score(112)
    progression=[
      ([59,62,66,69],[35,42],[59,62,66,69]),
      ([55,59,62,66],[31,38],[55,59,62,66]),
      ([57,62,66,69],[38,45],[57,62,66,69]),
      ([57,61,64,71],[33,40],[57,61,64,71])]
    phrase_a=[
      [(0,78,.75),(1,81,.5),(1.5,78,.5),(2,76,.75),(3,74,.75)],
      [(0,74,.75),(1,76,.5),(1.5,78,.5),(2,74,.75),(3,71,.75)],
      [(0,69,.5),(.5,74,.5),(1,78,1),(2,76,.5),(2.5,78,.5),(3,81,.75)],
      [(0,76,.5),(.5,73,.5),(1,71,.5),(1.5,73,.5),(2,76,1),(3,78,.75)]]
    phrase_b=[
      [(0,81,1),(1.5,78,.5),(2,76,.5),(2.5,74,.5),(3,78,.75)],
      [(0,79,.75),(1,78,.75),(2,74,.75),(3,71,.75)],
      [(0,78,.5),(.5,81,.5),(1,83,1),(2,81,.5),(2.5,78,.5),(3,76,.75)],
      [(0,76,.75),(1,73,.5),(1.5,76,.5),(2,78,1),(3,76,.75)]]
    for bar in range(24):
        base=bar*4; index=bar%4
        chord,bass,arp=progression[index]
        score.chord(base,0,chord,3.5,52)
        if index==3: score.chord(base+3.5,0,chord[1:],.25,37)
        score.add(base,2,bass[0],1.75,76)
        score.add(base+2,2,bass[1],1.5,69)
        if bar%2: score.add(base+3.5,2,bass[1],.25,51)
        for beat,j in [(0,0),(.5,2),(1.5,1),(2,0),(2.5,2),(3.5,3)]:
            score.add(base+beat,1,arp[j]+12,.25,54 if beat in (0,2) else 46)
        phrase=phrase_b if 8<=bar<16 else phrase_a
        for beat,n,duration in phrase[index]:
            if bar>=16 and beat==3 and index==3: n=74
            score.add(base+beat,3,n,duration,72 if beat==0 else 66)
        for beat in (0,2): score.add(base+beat,9,36,.25,49 if beat==0 else 40)
        for beat in (1,3): score.add(base+beat,9,38,.25,37)
        for k in range(8):
            if index==3 and k==7: continue
            score.add(base+k*.5,9,42,.25,32 if k%2 else 40)
        if bar%8==7:
            for beat in (3,3.5): score.add(base+beat,9,46,.25,28)
    duration=24*4*60/112
    a=score.render(duration+2.2,{0:4,1:24,2:35,3:11},
             {0:66,1:67,2:81,3:85,9:60},{0:57,1:96,2:64,3:40},True)
    n=round(duration*RATE)
    overlap=min(round(.8*RATE),len(a)-n)
    a[:overlap] += .55*a[n:n+overlap]
    a=a[:n]
    return master_v5(a,-20,True)

def build_ring():
    score=Score(112)
    for bar in (0,4):
        for beat,n,duration,vel in [(0,78,.5,87),(.5,81,.5,81),(1,85,.75,83),(2,83,1,82)]:
            score.add(bar+beat,3,n,duration,vel)
            score.add(bar+beat,1,n-12,duration,42)
        score.add(bar,0,59,2.5,36)
    duration=8*60/112
    a=score.render(duration,{0:4,1:12,3:11},{0:50,1:59,3:95},{0:64,1:97,3:43})
    return master_v5(a,-19,True)

def build_message():
    score=Score(138,v4=True)
    for beat,n,duration,vel in [(0,78,.17,73),(.19,81,.17,76),(.4,85,.31,68)]:
        score.add(beat,1,n,duration,vel)
        score.add(beat+.008,3,n-12,duration+.13,57)
    a=score.render(1.20,{1:11,3:12},{1:83,3:63},{1:48,3:85})
    return master_v4_notification(a)

def main():
    files=[
        ("CommsCraft-Pulse-hold-loop",build_hold()),
        ("CommsCraft-Beat-call-loop",build_ring()),
        ("CommsCraft-Signal-message-unchanged",build_message())]
    sha=[]
    for stem,audio in files:
        wav=OUT/(stem+".wav")
        sf.write(wav,audio,RATE,subtype="PCM_16")
        for suffix,options in [
            ("opus",["-c:a","libopus","-b:a","96k"]),
            ("mp3",["-c:a","libmp3lame","-q:a","3"])]:
            subprocess.run(["ffmpeg","-hide_banner","-loglevel","error","-y",
                "-i",str(wav),*options,str(OUT/(stem+"."+suffix))],check=True)
        for suffix in ("wav","opus","mp3"):
            path=OUT/(stem+"."+suffix)
            sha.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
        print(stem,len(audio)/RATE,"seconds")
    (OUT/"SHA256SUMS.txt").write_text("\n".join(sha)+"\n",encoding="utf-8")
if __name__=="__main__":
    main()
