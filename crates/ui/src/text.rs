//! The interface texts in German and English (chosen from the system language at start).

use crate::model::{Connection, Presence, Reason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    /// From a POSIX locale value (`de_DE.UTF-8` → German); English otherwise.
    pub fn from_locale(v: &str) -> Self {
        if v.to_ascii_lowercase().starts_with("de") {
            Lang::De
        } else {
            Lang::En
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            Lang::De => &DE,
            Lang::En => &EN,
        }
    }
}

macro_rules! texts {
    ($($name:ident: $de:expr, $en:expr;)*) => {
        /// All fixed texts of the interface.
        pub struct Texts { $(pub $name: &'static str,)* pub shortcut_lines: &'static [&'static str], lang: Lang }
        pub static DE: Texts = Texts { $($name: $de,)* shortcut_lines: SHORTCUTS_DE, lang: Lang::De };
        pub static EN: Texts = Texts { $($name: $en,)* shortcut_lines: SHORTCUTS_EN, lang: Lang::En };
    };
}

const SHORTCUTS_DE: &[&str] = &[
    "Strg+K  Suche",
    "Strg+N  Neue Unterhaltung",
    "Strg+1 … 4  Chats, Kontakte, Anrufe, Meetings",
    "Enter  Senden · Umschalt+Enter  Zeilenumbruch",
    "Esc  Dialog oder Suche schließen",
    "Tab / Pfeiltasten  Fokus bewegen",
];
const SHORTCUTS_EN: &[&str] = &[
    "Ctrl+K  Search",
    "Ctrl+N  New conversation",
    "Ctrl+1 … 4  Chats, contacts, calls, meetings",
    "Enter  Send · Shift+Enter  New line",
    "Esc  Close dialog or search",
    "Tab / arrow keys  Move focus",
];

texts! {
    tagline: "PROFESSIONELLE KOMMUNIKATION", "PROFESSIONAL COMMUNICATION";
    search_hint: "Personen, Chats und Kontakte suchen", "Search people, chats and contacts";
    search_shortcut: "Strg K", "Ctrl K";
    profile_menu: "Status und Abmelden", "Status and sign-out";
    my_status: "Mein Status", "My status";
    sign_out: "Abmelden", "Sign out";
    chats: "Chats", "Chats";
    contacts: "Kontakte", "Contacts";
    calls: "Anrufe", "Calls";
    meetings: "Meetings", "Meetings";
    settings: "Einstellungen", "Settings";
    mono_chats: "CH", "CH";
    mono_contacts: "KO", "CO";
    mono_calls: "AN", "CA";
    mono_meetings: "ME", "ME";
    mono_settings: "EI", "SE";
    hide_context: "Seitenleiste ausblenden", "Hide side panel";
    show_context: "Seitenleiste einblenden", "Show side panel";
    show_context_setting: "Rechte Seitenleiste anzeigen", "Show the right side panel";
    status_demo: "DEMO · Keine Verbindung zu einem echten Server", "DEMO · Not connected to a real server";
    status_connected: "Verbunden (TLS, angemeldet)", "Connected (TLS, signed in)";
    status_degraded: "Eingeschränkt", "Degraded";
    status_connecting: "Verbindung wird aufgebaut …", "Connecting …";
    status_disconnected: "Verbindung getrennt", "Disconnected";
    status_failed: "Anmeldung fehlgeschlagen", "Sign-in failed";
    status_signed_out: "Nicht angemeldet", "Not signed in";
    messages_title: "Nachrichten", "Messages";
    new_chat_tip: "Neue Unterhaltung (Strg+N)", "New conversation (Ctrl+N)";
    filter_all: "Alle", "All";
    filter_unread: "Ungelesen", "Unread";
    filter_groups: "Gruppen", "Groups";
    filter_online: "Online", "Online";
    filter_favorites: "Favoriten", "Favourites";
    filter_missed: "Verpasst", "Missed";
    favorites: "Favoriten", "Favourites";
    recent_conversations: "Letzte Unterhaltungen", "Recent conversations";
    no_conversations: "Noch keine Unterhaltungen. Mit + oder Strg+N eine beginnen.", "No conversations yet. Start one with + or Ctrl+N.";
    no_conversations_filter: "Keine Unterhaltung passt zum Filter.", "No conversation matches the filter.";
    typing_short: "schreibt …", "typing …";
    conversation_failed: "Unterhaltung nicht zustande gekommen", "Conversation could not be opened";
    conversation_ended: "Die Unterhaltung ist beendet.", "The conversation has ended.";
    conversation_closed_hint: "Die Unterhaltung ist beendet – neu beginnen über Kontakte oder +", "The conversation has ended – start again from contacts or +";
    opening: "Wird geöffnet …", "Opening …";
    you: "Sie", "You";
    yesterday: "Gestern", "Yesterday";
    today: "Heute", "Today";
    favorite: "Als Favorit merken", "Add to favourites";
    unfavorite: "Aus Favoriten entfernen", "Remove from favourites";
    end_conversation: "Unterhaltung beenden", "End conversation";
    select_conversation: "Keine Unterhaltung ausgewählt", "No conversation selected";
    select_conversation_hint: "Links eine Unterhaltung wählen oder über Kontakte einen Chat beginnen.", "Pick a conversation on the left or start a chat from contacts.";
    group_chat: "Gruppenchat", "Group chat";
    search_in_chat: "Im Chat suchen", "Search in chat";
    call: "Anrufen", "Call";
    sending: "Wird gesendet", "Sending";
    delivered: "Zugestellt", "Delivered";
    not_delivered: "Nicht zugestellt", "Not delivered";
    not_delivered_to: "Nicht zugestellt an", "Not delivered to";
    delivery_unknown: "Zustellung unbekannt", "Delivery unknown";
    demo_content_title: "Demo-Inhalt", "Demo content";
    demo_content: "Diese Nachrichten sind erfundene Beispieldaten. Nichts wird gesendet oder empfangen.", "These messages are made-up sample data. Nothing is sent or received.";
    no_messages: "Noch keine Nachrichten.", "No messages yet.";
    attach: "+ Datei", "+ File";
    formatting: "Formatierung", "Formatting";
    send: "Senden", "Send";
    send_tip: "Enter sendet, Umschalt+Enter fügt eine Zeile ein", "Enter sends, Shift+Enter adds a line";
    invite: "Einladen", "Invite";
    details: "Details", "Details";
    group_info: "Gruppeninformationen", "Group information";
    contact_info: "Kontaktinformationen", "Contact information";
    members_roles: "Mitglieder und Rollen", "Members and roles";
    no_roster: "Die Teilnehmerliste ist noch nicht eingetroffen.", "The participant list has not arrived yet.";
    presenter: "Referent", "Presenter";
    function: "Funktion", "Role";
    availability: "Verfügbarkeit", "Availability";
    files: "Dateien", "Files";
    history_title: "Verlauf", "History";
    history_body: "Nachrichten bleiben nur bis zum Abmelden in diesem Fenster; es gibt kein lokales Archiv und keine Volltextsuche.", "Messages stay in this window until sign-out only; there is no local archive and no full-text search.";
    delivery_title: "Zustellung", "Delivery";
    delivery_body: "„Zugestellt“ heißt: der Server oder die Gegenstelle hat die Nachricht angenommen. Gelesen ist damit nicht bestätigt.", "“Delivered” means the server or peer accepted the message. It does not confirm that it was read.";
    invite_title: "In die Gruppe einladen", "Invite to the group";
    transfer_title: "Anruf übertragen", "Transfer the call";
    new_chat_title: "Neue Unterhaltung", "New conversation";
    new_chat_body: "SIP-Adresse eingeben oder einen Kontakt wählen.", "Enter a SIP address or pick a contact.";
    address_hint: "name@firma.example", "name@company.example";
    cancel: "Abbrechen", "Cancel";
    start_chat: "Chat starten", "Start chat";
    start_group: "Als Gruppe starten", "Start as group";
    start_group_tip: "Eine Gruppenunterhaltung (Ad-hoc-Konferenz) öffnen und die Person einladen", "Open a group conversation (ad-hoc conference) and invite the person";
    contacts_sub: "Personen und Verfügbarkeit", "People and availability";
    other_contacts: "Weitere Kontakte", "Other contacts";
    no_contacts: "Die Kontaktliste ist leer.", "The contact list is empty.";
    contacts_offline: "Ohne Verbindung sind keine Kontakte verfügbar.", "Without a connection no contacts are available.";
    no_contacts_filter: "Kein Kontakt passt zum Filter.", "No contact matches the filter.";
    source_contact_list: "Kontaktliste", "Contact list";
    source_address_book: "Adressbuch", "Address book";
    source_directory: "Adressbuchsuche (Server)", "Address book search (server)";
    source_demo: "Demonstrationsdaten", "Demonstration data";
    select_contact: "Kein Kontakt ausgewählt", "No contact selected";
    select_contact_hint: "Links einen Kontakt wählen. Doppelklick startet einen Chat.", "Pick a contact on the left. A double click starts a chat.";
    audio_call: "Audioanruf", "Audio call";
    contact_overview: "Kontaktübersicht", "Contact overview";
    status: "Status", "Status";
    sip_address: "SIP-Adresse", "SIP address";
    phone: "Telefon", "Phone";
    origin: "Herkunft", "Source";
    presence_hint: "Präsenzmeldungen sind Momentaufnahmen. Ein grüner Status ist kein Zustell- oder Lesebeleg.", "Presence is a snapshot. A green status does not prove delivery or reading.";
    organisation: "Organisation", "Organisation";
    not_known: "Nicht bekannt", "Not known";
    reachability: "Erreichbarkeit", "Reachability";
    reachability_body: "Chat und Audioanruf laufen über die angemeldete Sitzung; ob die Person sie annimmt, zeigt erst die Antwort.", "Chat and audio calls go through the signed-in session; whether the person answers shows only in the reply.";
    availability_body: "Live-Präsenz nur, wenn eine gültige Servermeldung vorliegt.", "Live presence only when a valid server notification arrived.";
    note: "Hinweis", "Note";
    demo_person: "Demo-Person und Demo-Daten; keine echte Verbindung.", "Demo person and demo data; no real connection.";
    calls_sub: "Gespräche und Verlauf", "Calls and history";
    active_call: "Aktives Gespräch", "Active call";
    recent_calls: "Letzte Gespräche", "Recent calls";
    no_calls: "Noch keine Anrufe in dieser Sitzung.", "No calls in this session yet.";
    incoming: "Eingehend", "Incoming";
    outgoing: "Ausgehend", "Outgoing";
    missed: "Verpasst", "Missed";
    status_code: "Status", "status";
    call_back_tip: "Doppelklick ruft zurück", "Double click to call back";
    call_dialing: "Wird gewählt …", "Dialling …";
    call_ringing: "Es klingelt …", "Ringing …";
    call_incoming: "Eingehender Anruf", "Incoming call";
    call_on_hold: "Gehalten", "On hold";
    call_connected: "Audio verbunden", "Audio connected";
    call_ended: "Beendet", "Ended";
    no_active_call: "Kein laufender Anruf", "No call in progress";
    no_active_call_hint: "Anrufe beginnen über Kontakte oder den Chat-Kopf.", "Start calls from contacts or the chat header.";
    demo_call: "Demo – kein echter Anruf, kein Ton", "Demo – no real call, no sound";
    answer: "Annehmen", "Answer";
    decline: "Ablehnen", "Decline";
    mono_mute: "MI", "MI";
    mono_hold: "HA", "HO";
    mono_video: "VI", "VI";
    mono_transfer: "ÜB", "TR";
    mute: "Stummschalten", "Mute";
    unmute: "Ton an", "Unmute";
    hold: "Halten", "Hold";
    resume: "Fortsetzen", "Resume";
    video: "Video", "Video";
    transfer: "Übertragen", "Transfer";
    hang_up: "Anruf beenden", "End call";
    call_hint_title: "Hinweis zum Anrufstatus", "About the call status";
    call_hint: "„Verbunden“ erscheint erst, wenn die Sitzung die Antwort der Gegenstelle gemeldet hat; Wählen und Klingeln werden getrennt angezeigt.", "“Connected” appears only once the session reported the peer's answer; dialling and ringing are shown separately.";
    call_hint_demo: "Anrufstatus, Timer, Halten und Stummschalten sind hier Demo-Daten; es wird nichts gewählt.", "Call state, timer, hold and mute are demo data here; nothing is dialled.";
    call_details: "Gesprächsdetails", "Call details";
    call_kind: "Gesprächsart", "Call type";
    call_kind_audio: "Audio · 1:1", "Audio · 1:1";
    call_kind_conference: "Audio · Konferenz", "Audio · conference";
    participants: "Teilnehmer", "Participants";
    connection_state: "Verbindungsstatus", "Connection state";
    audio_devices: "Audiogeräte", "Audio devices";
    audio_devices_body: "Mikrofon und Lautsprecher über ALSA bzw. PipeWire (Standardgerät des Systems).", "Microphone and speaker through ALSA or PipeWire (the system's default device).";
    privacy: "Datenschutz", "Privacy";
    privacy_body: "Signalisierung über TLS, Medien über SRTP. Das ist keine Ende-zu-Ende-Verschlüsselung.", "Signalling over TLS, media over SRTP. This is not end-to-end encryption.";
    meetings_sub: "Besprechungen und Einladungen", "Meetings and invitations";
    refresh: "Aktualisieren", "Refresh";
    invitations: "Einladungen", "Invitations";
    invited_by: "Eingeladen von", "Invited by";
    join: "Teilnehmen", "Join";
    my_meetings: "Meine Besprechungen", "My meetings";
    no_meetings: "Keine Besprechungen bekannt.", "No meetings known.";
    untitled_meeting: "Besprechung ohne Titel", "Untitled meeting";
    valid_until: "Gültig bis", "Valid until";
    demo_meeting: "Demo-Besprechung – keine echte Konferenz", "Demo meeting – no real conference";
    conference_id: "Konferenz-ID", "Conference ID";
    meetings_title: "Besprechungen", "Meetings";
    meetings_hint: "Eine Besprechung links wählen oder mit „Jetzt besprechen“ eine Gruppe öffnen.", "Pick a meeting on the left or open a group with “Meet now”.";
    meet_now: "Jetzt besprechen", "Meet now";
    meet_now_tip: "Öffnet eine Gruppenunterhaltung, in die Personen eingeladen werden können", "Opens a group conversation people can be invited to";
    meeting_content_title: "Inhalte in Besprechungen", "Meeting content";
    meeting_content_body: "Bildschirmfreigabe ansehen, Whiteboard, Umfragen und Persistent Chat bietet zurzeit nur das Kommandozeilenwerkzeug.", "Viewing shared screens, whiteboard, polls and persistent chat are offered only by the command-line tool for now.";
    meeting_details: "Besprechungsdetails", "Meeting details";
    meeting_audio: "Audio", "Audio";
    meeting_audio_body: "Konferenzaudio läuft über die angemeldete Sitzung.", "Conference audio runs through the signed-in session.";
    meeting_sharing: "Freigaben", "Sharing";
    meeting_sharing_body: "Ansehen geteilter Bildschirme ist in dieser Oberfläche noch nicht verfügbar.", "Viewing shared screens is not available in this interface yet.";
    persistent_chat: "Persistent Chat", "Persistent chat";
    search_results: "Suche", "Search";
    search_scope: "Durchsucht Kontakte, Titel offener Unterhaltungen und – wenn verfügbar – das Adressbuch des Servers.", "Searches contacts, titles of open conversations and – when available – the server's address book.";
    conversations: "Unterhaltungen", "Conversations";
    nothing_found: "Nichts gefunden.", "Nothing found.";
    search_more_letters: "Ab drei Zeichen wird das Adressbuch durchsucht.", "The address book is searched from three characters on.";
    no_message_search: "Nachrichtentexte werden nicht durchsucht: es gibt kein Archiv und keinen Index.", "Message texts are not searched: there is no archive and no index.";
    display: "Darstellung", "Display";
    high_contrast: "Hoher Kontrast", "High contrast";
    scale: "Skalierung", "Scale";
    account: "Konto", "Account";
    shortcuts: "Tastenkürzel", "Keyboard shortcuts";
    capabilities_title: "Funktionen und ihr Stand", "Functions and their state";
    capabilities_body: "Was diese Oberfläche jetzt anbietet. „Vorschau“ ist nur Gestaltung mit Demo-Daten und hat keine Wirkung.", "What this interface offers now. “Preview” is design with demo data only and has no effect.";
    gate_available: "Verfügbar", "Available";
    gate_preview: "Vorschau", "Preview";
    gate_unavailable: "Nicht verfügbar", "Unavailable";
    cap_chat: "Einzelchat", "One-to-one chat";
    cap_group_chat: "Gruppenchat (Ad-hoc-Konferenz)", "Group chat (ad-hoc conference)";
    cap_presence: "Präsenz", "Presence";
    cap_directory: "Adressbuchsuche", "Address book search";
    cap_audio: "Audioanruf", "Audio call";
    cap_hold: "Halten", "Hold";
    cap_transfer: "Übertragen", "Transfer";
    cap_video: "Videoanruf", "Video call";
    cap_meetings: "Besprechungen", "Meetings";
    cap_files: "Dateiübertragung", "File transfer";
    cap_rich_text: "Formatierter Text", "Formatted text";
    cap_persistent_chat: "Persistent Chat (Räume)", "Persistent chat (rooms)";
    cap_history_search: "Suche im Verlauf", "History search";
    cap_read_receipts: "Lesebestätigung", "Read receipts";
    security_title: "Sicherheit", "Security";
    security_body: "Verbindungen nur mit gültigem TLS-Zertifikat, kein unverschlüsselter Rückfall. Gesichert heißt nicht Ende-zu-Ende-verschlüsselt. Chats werden nicht auf dem Datenträger gespeichert.", "Connections only with a valid TLS certificate, no unencrypted fallback. Secured does not mean end-to-end encrypted. Chats are not stored on disk.";
    preview_hint: "Vorschau mit Demo-Daten – ohne Wirkung", "Preview with demo data – no effect";
    sign_in_title: "Anmelden", "Sign in";
    sign_in_body: "Bei Skype for Business Server Ihrer Organisation.", "To your organisation's Skype for Business Server.";
    sign_in_address: "SIP-Adresse", "SIP address";
    sign_in_user: "Benutzername", "User name";
    sign_in_user_hint: "z. B. mkleinert", "e.g. mkleinert";
    sign_in_domain: "Windows-Domäne (optional)", "Windows domain (optional)";
    sign_in_domain_hint: "z. B. FIRMA", "e.g. COMPANY";
    sign_in_password: "Kennwort", "Password";
    sign_in_server: "Server (optional)", "Server (optional)";
    sign_in_server_hint: "host:port – leer: automatisch suchen", "host:port – empty: find automatically";
    sign_in: "Anmelden", "Sign in";
    sign_in_again: "Neu anmelden", "Sign in again";
    trust_title: "Server außerhalb Ihrer Domäne", "Server outside your domain";
    trust_no: "Nicht vertrauen", "Do not trust";
    sign_in_kerberos: "Kerberos versuchen (Windows-Domäne)", "Try Kerberos (Windows domain)";
    sign_in_kerberos_tip: "Nutzt ein vorhandenes Kerberos-Ticket des Systems, sonst das Kennwort; NTLM bleibt der Rückfall.", "Uses the system's Kerberos ticket if there is one, else the password; NTLM stays the fallback.";
    signing_in: "Anmeldung läuft …", "Signing in …";
    start_demo: "Demo ansehen", "View demo";
    start_demo_tip: "Oberfläche mit erfundenen Daten, ohne Server", "The interface with made-up data, without a server";
    sign_in_note: "Das Kennwort wird nicht gespeichert. Zertifikate werden immer geprüft.", "The password is not stored. Certificates are always checked.";
}

impl Texts {
    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn presence(&self, p: Presence) -> &'static str {
        let de = self.lang == Lang::De;
        match p {
            Presence::Available => {
                if de {
                    "Verfügbar"
                } else {
                    "Available"
                }
            }
            Presence::Busy => {
                if de {
                    "Beschäftigt"
                } else {
                    "Busy"
                }
            }
            Presence::InACall => {
                if de {
                    "Im Gespräch"
                } else {
                    "In a call"
                }
            }
            Presence::DoNotDisturb => {
                if de {
                    "Nicht stören"
                } else {
                    "Do not disturb"
                }
            }
            Presence::BeRightBack => {
                if de {
                    "Bin gleich zurück"
                } else {
                    "Be right back"
                }
            }
            Presence::Away => {
                if de {
                    "Abwesend"
                } else {
                    "Away"
                }
            }
            Presence::Offline => "Offline",
            Presence::Unknown => {
                if de {
                    "Status unbekannt"
                } else {
                    "Status unknown"
                }
            }
        }
    }

    pub fn reason(&self, r: Reason) -> &'static str {
        let de = self.lang == Lang::De;
        match r {
            Reason::NotConnected => {
                if de {
                    "Nicht verfügbar: keine Verbindung zum Server."
                } else {
                    "Unavailable: not connected to the server."
                }
            }
            Reason::NotImplemented => {
                if de {
                    "Im aktuellen Stand nicht implementiert."
                } else {
                    "Not implemented at this stage."
                }
            }
            Reason::CliOnly => {
                if de {
                    "In dieser Oberfläche noch nicht verfügbar; das Kommandozeilenwerkzeug bietet es an."
                } else {
                    "Not in this interface yet; the command-line tool offers it."
                }
            }
            Reason::ServerDisabled => {
                if de {
                    "Vom Server für dieses Konto nicht freigegeben."
                } else {
                    "Not enabled by the server for this account."
                }
            }
            Reason::NoDevice => {
                if de {
                    "Kein passendes Gerät gefunden."
                } else {
                    "No suitable device found."
                }
            }
            Reason::DemoOnly => {
                if de {
                    "In der Demo nicht vorgeführt."
                } else {
                    "Not shown in the demo."
                }
            }
        }
    }

    pub fn connection_short(&self, c: &Connection, demo: bool) -> &'static str {
        let de = self.lang == Lang::De;
        if demo {
            return if de { "Offline-Demo" } else { "Offline demo" };
        }
        match c {
            Connection::Connected => {
                if de {
                    "Angemeldet"
                } else {
                    "Signed in"
                }
            }
            Connection::Degraded(_) => {
                if de {
                    "Eingeschränkt"
                } else {
                    "Degraded"
                }
            }
            Connection::Connecting => {
                if de {
                    "Verbinde …"
                } else {
                    "Connecting …"
                }
            }
            Connection::Disconnected(_) => {
                if de {
                    "Getrennt"
                } else {
                    "Disconnected"
                }
            }
            Connection::Failed(_) => {
                if de {
                    "Fehler"
                } else {
                    "Failed"
                }
            }
            Connection::SignedOut | Connection::Demo => {
                if de {
                    "Nicht angemeldet"
                } else {
                    "Signed out"
                }
            }
        }
    }

    pub fn members_count(&self, n: usize) -> String {
        match (self.lang, n) {
            (Lang::De, 1) => "1 Mitglied".into(),
            (Lang::De, n) => format!("{n} Mitglieder"),
            (Lang::En, 1) => "1 member".into(),
            (Lang::En, n) => format!("{n} members"),
        }
    }

    pub fn more_members(&self, n: usize) -> String {
        match self.lang {
            Lang::De => format!("und {n} weitere"),
            Lang::En => format!("and {n} more"),
        }
    }

    pub fn unread_count(&self, n: usize) -> String {
        match self.lang {
            Lang::De => format!("{n} ungelesen"),
            Lang::En => format!("{n} unread"),
        }
    }

    pub fn is_typing(&self, who: &str) -> String {
        match self.lang {
            Lang::De => format!("{who} schreibt …"),
            Lang::En => format!("{who} is typing …"),
        }
    }

    pub fn trust_body(
        &self,
        host: &str,
        user_domain: &str,
        named_by: &str,
        domain: &str,
    ) -> String {
        match self.lang {
            Lang::De => format!("Der Server {host} gehört nicht zu Ihrer SIP-Domäne {user_domain}. Er wurde von {named_by} über eine geprüfte HTTPS-Verbindung genannt. Wenn Sie Servern in {domain} vertrauen, geht Ihre Anmeldung (und das Kennwort) dorthin; die Entscheidung wird gespeichert."),
            Lang::En => format!("The server {host} is not in your SIP domain {user_domain}. It was named by {named_by} over a verified HTTPS connection. If you trust servers in {domain}, your sign-in (and password) goes there; the decision is remembered."),
        }
    }

    pub fn trust_yes(&self, domain: &str) -> String {
        match self.lang {
            Lang::De => format!("{domain} vertrauen"),
            Lang::En => format!("Trust {domain}"),
        }
    }

    pub fn write_to(&self, who: &str) -> String {
        match self.lang {
            Lang::De => format!("Nachricht an {who} schreiben …"),
            Lang::En => format!("Write to {who} …"),
        }
    }
}
