use anyhow::Result;
use base64::Engine;
use qrcode_generator::QrCodeEcc;
pub fn gen_qr_data_uri(d: &str) -> Result<String> {
    let b = qrcode_generator::to_png_to_vec(d, QrCodeEcc::Low, 320)?;
    // 对 b 进行 base64 编码
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(b)
    ))
}
