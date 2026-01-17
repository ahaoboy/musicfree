pub fn get_md5(s: &str) -> String {
    let digest = md5::compute(s);
    format!("{:x}", digest)
}
