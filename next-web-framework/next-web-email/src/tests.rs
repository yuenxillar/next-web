#[cfg(test)]
mod tests {
    use super::*;
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    #[test]
    fn emil_test() {
        let email = Message::builder()
            .from("NoBody <yuenxillar@163.com>".parse().unwrap())
            .reply_to("Yuin <yuenxillar@163.com>".parse().unwrap())
            .to("Hei <yuenxillar@163.com>".parse().unwrap())
            .subject("Happy new year")
            .header(ContentType::TEXT_HTML)
            .body(String::from("<html><body><h1>Be happy!</h1></body></html>"))
            .unwrap();

        let creds = Credentials::new("yuenxillar".to_owned(), "5201314Apink#".to_owned());

        // Open a remote connection to gmail
        let mailer = SmtpTransport::relay("smtp.163.com")
            .unwrap()
            .credentials(creds)
            .build();

        // Send the email
        match mailer.send(&email) {
            Ok(_) => println!("Email sent successfully!"),
            Err(e) => panic!("Could not send email: {e:?}"),
        }
    }
}
