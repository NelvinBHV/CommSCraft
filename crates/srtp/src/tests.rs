use super::*;

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn arr<const N: usize>(s: &str) -> [u8; N] {
    hex(s).try_into().unwrap()
}

/// RFC 3711 appendix B.3 (IE-09).
#[test]
fn rfc3711_b3_key_derivation() {
    let key: [u8; 16] = arr("E1F97A0D3E018BE0D64FA32C06DE4139");
    let salt: [u8; 14] = arr("0EC675AD498AFEEBB6960B3AABE6");
    let k = SessionKeys::derive(&key, &salt, false);
    assert_eq!(
        k.encryption.to_vec(),
        hex("C61E7A93744F39EE10734AFE3FF7A087")
    );
    assert_eq!(k.salt.to_vec(), hex("30CBBC08863D8C85D49DB34A9AE1"));
    assert_eq!(
        k.authentication.to_vec(),
        hex("CEBE321F6FF7716B6FD4AB49AF256A156D38BAA4")
    );
}

const MASTER_KEY: &str = "CB4A3C93F3D587ABA1AB0BDF8C6AA0FB";
const MASTER_SALT: &str = "53EF4F4594296D0EB286D9CC96E4";

/// [MS-SSRTP] §4.1, p. 18.
#[test]
fn ssrtp_key_derivation() {
    let rtp = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), false);
    assert_eq!(
        rtp.encryption.to_vec(),
        hex("C3FCC67BFBF17CFA2DC69F4B4CFC59CD")
    );
    assert_eq!(
        rtp.authentication.to_vec(),
        hex("23B8B2D911CF8C6416F4AAB94083E0CC32615694")
    );
    assert_eq!(rtp.salt.to_vec(), hex("929B3AD0FDB565FDBEAA50412C8D"));
    let rtcp = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), true);
    assert_eq!(
        rtcp.encryption.to_vec(),
        hex("122E3C94A0D945242AF0B79C6EDCE0BB")
    );
    assert_eq!(
        rtcp.authentication.to_vec(),
        hex("999BDAC078DBC12E7677AD05B9B2B54CBFDCBAA6")
    );
    assert_eq!(rtcp.salt.to_vec(), hex("839D270762975E43F6351493434E"));
}

const PAYLOAD: &str = "3F68B92587D38C18D22AFA3FCF30B63098BDB1213F30F91054911E0521EE3A8EE386794C5B5F
D4B9A6477719F27937B6A0C7E8221250A57C5A42E8A99565F7559F21998F2555003F4677DB4AFCD3
59738B51D538B4BE1780CC618E686E9862343F0C65D5A86C334B1915B48D99FCAD8E39E9C8F9BD6
915FD7CBBFFD94A73F373615C5CC8C827B2E4C33EEB492D38";

const ENCRYPTED: &str =
    "C1D49FFD5B845AAC755FCE604A2B9225D672DDB5A3C4664447F3D39D841B6C84373437FAED0
11C30AD1D91FB9CC7CF1796A97D99886EBB694E6C050ED100073D2526C9FC56AB08555B3A1A2589
D1491D0402EB79C1C1C6E439C815B4AB83421F57293008B70AB296DAFFD7E6E2E67E6A93FF89FE8C
DE14C49FBAB13E233793B1934AA8A5BDBC3BD6B0A91D520EC9";

/// [MS-SSRTP] §4.2, pp. 18–19.
#[test]
fn ssrtp_packet_transform() {
    let keys = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), false);
    let header = hex("80728001AE773346DE1A3236");
    let payload = hex(PAYLOAD);
    assert_eq!(payload.len(), 142);
    let out = ssrtp_protect(&keys, &header, &payload, 0x5E1A_3236_8001, 2).unwrap();
    assert_eq!(out.encrypted_payload, hex(ENCRYPTED));
    let mut expected_block = hex(ENCRYPTED);
    expected_block.extend(hex("5E1A32368001"));
    expected_block.resize(192, 0);
    expected_block.extend(hex("80728001AE773346DE1A3236"));
    expected_block.extend(hex("00000002"));
    assert_eq!(out.authenticated_block.len(), 208);
    assert_eq!(out.authenticated_block, expected_block);
    assert_eq!(out.tag.to_vec(), hex("2FA5BAC13AC58423BE4A"));
}

#[test]
fn rejects_malformed_input() {
    let keys = SessionKeys::derive(&[0; 16], &[0; 14], false);
    assert_eq!(
        ssrtp_protect(&keys, &[0x80], &[], 0, 0),
        Err(SrtpError::MalformedHeader)
    );
    let header_with_missing_csrc = hex("81728001AE773346DE1A3236");
    assert_eq!(
        ssrtp_protect(&keys, &header_with_missing_csrc, &[], 0, 0),
        Err(SrtpError::MalformedHeader)
    );
    assert_eq!(
        ssrtp_protect(&keys, &hex("80728001AE773346DE1A3236"), &[], MAX_ESN + 1, 0),
        Err(SrtpError::EsnOutOfRange)
    );
}
