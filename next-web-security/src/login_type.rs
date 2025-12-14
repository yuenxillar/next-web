#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginType<T = ()>  {
    Username,
    Mobile,
    Email,
    Wechat,
    QQ,
    Weibo,
    Alipay,
    Baidu,
    Tencent,
    Facebook,
    Twitter,
    GitHub,
    GitLab,
    StackOverflow,
    YouTube,
    Apple,
    Microsoft,
    Samsung,
    Google,
    Any(T)
}