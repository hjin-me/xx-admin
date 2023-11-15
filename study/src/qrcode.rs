use anyhow::{anyhow, Result};
pub fn decode_qr(b: &[u8]) -> Result<String> {
    let img = image::load_from_memory(b)?;

    let decoder = bardecoder::default_decoder();

    let results = decoder.decode(&img);
    let first = match results.get(0) {
        Some(r) => r.as_ref(),
        None => return Err(anyhow!("没有识别到二维码")),
    };
    Ok(match first {
        Ok(r) => r.clone(),
        Err(e) => return Err(anyhow!("识别二维码失败: {}", e)),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_q() {
        _ = dbg!(std::env::current_dir());
        let b = include_bytes!("./qr.png");
        let r = decode_qr(b).unwrap();

        assert_eq!(r.as_str(), "https://login.xuexi.cn/login/qrcommit?showmenu=false&code=qr:E7298D91-9D75-44EF-BCBE-CAB558A92158&appId=dingoankubyrfkttorhpou");
    }
}
