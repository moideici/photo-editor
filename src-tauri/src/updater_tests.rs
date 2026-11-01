use base64::{Engine,engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey,Signature};
#[test]
fn official_channel_requires_https_and_the_actual_public_key(){
    let config:serde_json::Value=serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(config["bundle"]["createUpdaterArtifacts"],true);
    assert_eq!(config["plugins"]["updater"]["endpoints"],serde_json::json!(["https://github.com/moidefr/photo-editor/releases/latest/download/latest.json"]));
    let text=String::from_utf8(STANDARD.decode(config["plugins"]["updater"]["pubkey"].as_str().unwrap()).unwrap()).unwrap();
    PublicKey::decode(&text).unwrap();
}
#[test]
#[ignore="requires freshly signed NSIS and MSI in PRISME_UPDATE_DIR"]
fn real_update_installers_reject_tampering_wrong_key_and_bad_signature(){
    let root=std::path::PathBuf::from(std::env::var_os("PRISME_UPDATE_DIR").expect("PRISME_UPDATE_DIR"));
    let config:serde_json::Value=serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let public=String::from_utf8(STANDARD.decode(config["plugins"]["updater"]["pubkey"].as_str().unwrap()).unwrap()).unwrap();
    let key=PublicKey::decode(&public).unwrap();let mut lines=public.lines();let comment=lines.next().unwrap();let mut binary=STANDARD.decode(lines.next().unwrap()).unwrap();binary[41]^=1;
    let wrong=PublicKey::decode(&format!("{comment}\n{}\n",STANDARD.encode(binary))).unwrap();
    let mut files=Vec::new();for dir in ["nsis","msi"]{for entry in std::fs::read_dir(root.join(dir)).unwrap(){let file=entry.unwrap().path();if file.extension().is_some_and(|e|e=="sig"){files.push(file);}}}
    assert_eq!(files.len(),2,"Require both installer signatures in an isolated directory");
    for file in &files {
        let artifact=file.with_extension("");let bytes=std::fs::read(&artifact).unwrap();
        let signature_bytes=STANDARD.decode(std::fs::read_to_string(file).unwrap().trim()).unwrap();let text=String::from_utf8(signature_bytes).unwrap();let signature=Signature::decode(&text).unwrap();
        key.verify(&bytes,&signature,false).unwrap();
        let mut changed=bytes.clone();let middle=changed.len()/2;changed[middle]^=1;
        assert!(key.verify(&changed,&signature,false).is_err());assert!(wrong.verify(&bytes,&signature,false).is_err());
        let mut lines=text.lines().map(str::to_owned).collect::<Vec<_>>();let mut raw=STANDARD.decode(&lines[1]).unwrap();raw[20]^=1;lines[1]=STANDARD.encode(raw);let invalid=Signature::decode(&lines.join("\n")).unwrap();assert!(key.verify(&bytes,&invalid,false).is_err());
    }
    let report=serde_json::json!({"passed":true,"installers":files.iter().map(|p|p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>(),"checksPerInstaller":["actual release signature accepted","modified installer rejected","different public key rejected","corrupted signature rejected"],"doesNotProve":["network retry","actual installation","upgrade","Windows Authenticode publisher certificate"]});
    std::fs::write(root.join("signature-verification.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
