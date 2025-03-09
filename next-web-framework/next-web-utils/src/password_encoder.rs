/**
*struct:    PasswordEncoder
*desc:      用户密码加密解密
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct PasswordEncoder;

impl PasswordEncoder {
    pub fn md5_encode(password: &str) -> String {
        let digest = md5::compute(password);
        format!("{:x}", digest)
    }

    pub fn md5_verify(password: &str, raw_password: &str) -> bool {
        PasswordEncoder::md5_encode(password).eq(&raw_password)
    }
}
