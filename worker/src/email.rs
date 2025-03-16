//! Email module for sending and receiving email messages.
//!
//! Note: Some tests for this module require wasm_bindgen, and won't run on the native target.
//! Specifically, conversion tests for [`EmailMessage`] and [`RawEmailMessage`] require wasm_bindgen.
use std::{convert::TryFrom, fmt::Display};

use bon::bon;
use futures_util::TryStreamExt;
use wasm_bindgen_futures::JsFuture;
use web_sys::ReadableStream;
use worker_sys::EmailMessage as EmailMessageSys;

use crate::{send::SendFuture, ByteStream, Headers, Result};

pub struct EmailMessage {
    pub inner: EmailMessageSys,
}

#[bon]
impl EmailMessage {
    /// Construct a new email message
    ///
    /// # Example
    ///
    /// ```rust
    /// use worker::{EmailMessage, RawEmailMessage};
    ///
    /// let from_email = "source@example.com";
    /// let to_email = "destination@example.com";
    /// let subject = "Hello, World!";
    /// let date = "Sat, 15 Mar 2025 22:06:02 +0000";
    /// let message_id = "<message_id>";
    /// let message = "Hello, World!";
    ///
    /// let msg = RawEmailMessage::builder()
    ///   .from_email(from_email)
    ///   .to_email(to_email)
    ///   .subject(subject)
    ///   .date(date)
    ///   .message_id(message_id)
    ///   .message(message)
    ///   .build()
    ///   .to_string();
    ///
    /// let email = EmailMessage::builder()
    ///    .from("source@example.com")
    ///    .to("destination@email.com")
    ///    .raw(msg)
    ///    .build();
    /// ```
    #[builder]
    pub fn new(from: &str, to: &str, raw: &str) -> Result<Self> {
        Ok(EmailMessage {
            inner: EmailMessageSys::new(from, to, raw)?,
        })
    }

    /// construct a new email message for a ReadableStream
    pub fn new_from_stream(from: &str, to: &str, raw: &ReadableStream) -> Result<Self> {
        Ok(EmailMessage {
            inner: EmailMessageSys::new_from_stream(from, to, raw)?,
        })
    }

    /// the from field of the email message
    pub fn from_email(&self) -> String {
        self.inner.from().unwrap().into()
    }

    /// the to field of the email message
    pub fn to_email(&self) -> String {
        self.inner.to().unwrap().into()
    }

    /// the headers field of the email message
    pub fn headers(&self) -> Headers {
        Headers(self.inner.headers().unwrap())
    }

    /// the raw email message
    pub fn raw(&self) -> Result<ByteStream> {
        self.inner.raw().map_err(Into::into).map(|rs| ByteStream {
            inner: wasm_streams::ReadableStream::from_raw(rs).into_stream(),
        })
    }

    pub async fn raw_bytes(&self) -> Result<Vec<u8>> {
        self.raw()?
            .try_fold(Vec::new(), |mut bytes, mut chunk| async move {
                bytes.append(&mut chunk);
                Ok(bytes)
            })
            .await
    }

    /// the raw size of the message
    pub fn raw_size(&self) -> f64 {
        self.inner.raw_size().unwrap().into()
    }

    /// reject message with reason
    pub fn reject(&self, reason: String) {
        self.inner.set_reject(reason.into()).unwrap()
    }

    /// forward message to recipient
    pub async fn forward(&self, recipient: String, headers: Option<Headers>) -> Result<()> {
        let promise = self.inner.forward(recipient.into(), headers.map(|h| h.0))?;

        let fut = SendFuture::new(JsFuture::from(promise));
        fut.await?;
        Ok(())
    }

    /// reply with email message to recipient
    pub async fn reply(&self, message: EmailMessage) -> Result<()> {
        let promise = self.inner.reply(message.inner)?;

        let fut = SendFuture::new(JsFuture::from(promise));
        fut.await?;
        Ok(())
    }
}

/// Raw message builder uses the type system to build raw email messages.
///
/// # Example
///
/// ```rust
/// let msg_built = RawEmailMessage::builder()
///    // mandatory fields
///    .from_email("bot@cloudflare.com")
///    .from_name("Cloudflare bot")
///    .to_email("toon@example.com")
///    .to_name("Toon")
///    .subject("Email well received!")
///    .date("Sat, 15 Mar 2025 22:06:02 +0000")
///    // optional fields
///    .cc_email("another@example.com")
///    .cc_name("Another User")
///    .bcc_email("hidden@example.com")
///    .bcc_name("Hidden User")
///    .build();
/// ```
///
/// ## Mandatory Fields:
///
/// From: This field specifies the sender's email address.
/// To: This field specifies the recipient's email address.
/// Subject: This field specifies the subject of the email.
/// Date: This field specifies the date and time the email was sent. It must follow a specific format as per the Internet Message Format standard (RFC 5322): Date: <day-of-week>, <day> <month> <year> <hour>:<minute>:<second> <timezone>.
/// Message-ID: This is a unique identifier for the email message.
///
/// ## Optional Fields:
///
/// Cc: This field specifies additional recipients who will receive a copy of the email.
/// Bcc: This field specifies recipients who will receive a blind copy of the email (other recipients will not see these addresses).
/// In-Reply-To: This field references the Message-ID of the email being replied to, used in threading email conversations.
/// References: This field contains the Message-ID of the previous emails in a thread.
/// Reply-To: This field specifies an email address where replies should be sent, different from the sender's address.
/// Content-Type: This field specifies the MIME type of the email content, which is important for formatting (e.g., text/plain, text/html).
///
/// ## Optinoal Custom Headers
///
/// X-Original-To: Indicates the original recipient address before any forwarding.
/// X-Mailer: Specifies the software used to send the email.
/// X-Priority: Specifies the priority of the email (e.g., 1 for highest priority).
/// X-MSMail-Priority: Similar to X-Priority, used by Microsoft email clients.
/// X-Spam-Status: Indicates if the email has been flagged as spam.
/// X-Spam-Score: Provides a numeric score indicating the likelihood of the email being spam.
/// X-Spam-Flag: Indicates whether the email is flagged as spam (YES or NO).
/// X-Spam-Report: Provides a detailed report on the spam score.
/// X-Spam-Level: Provides a visual representation of the spam score.
/// X-Spam-Checker-Version: Specifies the version of the spam checker used.
#[derive(Debug)]
pub struct RawEmailMessage<'a> {
    // Mandatory fields
    from_email: &'a str,
    from_name: &'a str,
    to_email: &'a str,
    to_name: &'a str,
    subject: &'a str,
    date: &'a str,
    message: &'a str,
    message_id: &'a str,
    // Optional fields
    cc_email: Option<&'a str>,
    cc_name: Option<&'a str>,
    bcc_email: Option<&'a str>,
    bcc_name: Option<&'a str>,
    in_reply_to: Option<&'a str>,
    references: Option<&'a str>,
    reply_to: Option<&'a str>,
    content_type: Option<&'a str>,
    x_original_to: Option<&'a str>,
    x_mailer: Option<&'a str>,
    x_priority: Option<&'a str>,
    x_msmail_priority: Option<&'a str>,
    x_spam_status: Option<&'a str>,
    x_spam_score: Option<&'a str>,
    x_spam_flag: Option<&'a str>,
    x_spam_report: Option<&'a str>,
    x_spam_level: Option<&'a str>,
    x_spam_checker_version: Option<&'a str>,
}

#[bon]
impl<'a> RawEmailMessage<'a> {
    /// Construct a new email message
    ///
    /// Takes string slices for all fields.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let msg_built = RawEmailMessage::builder()
    ///    // mandatory fields
    ///    .from_email("bot@cloudflare.com")
    ///    .from_name("Cloudflare bot")
    ///    .to_email("toon@example.com")
    ///    .to_name("Toon")
    ///    .subject("Email well received!")
    ///    .date("Sat, 15 Mar 2025 22:06:02 +0000")
    ///    // optional fields
    ///    .cc_email("another@example.com")
    ///    .cc_name("Another User")
    ///    .bcc_email("hidden@example.com")
    ///    .bcc_name("Hidden User")
    ///    .build();
    ///
    /// let email = EmailMessage::try_from(msg_built).unwrap();
    /// ```
    #[builder]
    pub fn new(
        from_email: &'a str,
        from_name: &'a str,
        to_email: &'a str,
        to_name: &'a str,
        subject: &'a str,
        date: &'a str,
        message_id: &'a str,
        message: &'a str,
        cc_email: Option<&'a str>,
        cc_name: Option<&'a str>,
        bcc_email: Option<&'a str>,
        bcc_name: Option<&'a str>,
        in_reply_to: Option<&'a str>,
        references: Option<&'a str>,
        reply_to: Option<&'a str>,
        content_type: Option<&'a str>,
        x_original_to: Option<&'a str>,
        x_mailer: Option<&'a str>,
        x_priority: Option<&'a str>,
        x_msmail_priority: Option<&'a str>,
        x_spam_status: Option<&'a str>,
        x_spam_score: Option<&'a str>,
        x_spam_flag: Option<&'a str>,
        x_spam_report: Option<&'a str>,
        x_spam_level: Option<&'a str>,
        x_spam_checker_version: Option<&'a str>,
    ) -> Self {
        RawEmailMessage {
            from_email,
            from_name,
            to_email,
            to_name,
            subject,
            date,
            message_id,
            message,
            cc_email,
            cc_name,
            bcc_email,
            bcc_name,
            in_reply_to,
            references,
            reply_to,
            content_type,
            x_original_to,
            x_mailer,
            x_priority,
            x_msmail_priority,
            x_spam_status,
            x_spam_score,
            x_spam_flag,
            x_spam_report,
            x_spam_level,
            x_spam_checker_version,
        }
    }
}

impl Display for RawEmailMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut msg = format!(
            "From: {} <{}>\nTo: {} <{}>\nSubject: {}\nDate: {}\nMessage-ID: {}\n",
            self.from_name,
            self.from_email,
            self.to_name,
            self.to_email,
            self.subject,
            self.date,
            self.message_id
        );

        if let Some(cc_email) = self.cc_email {
            msg.push_str(&format!(
                "Cc: {} <{}>\n",
                self.cc_name.unwrap_or(""),
                cc_email
            ));
        }

        if let Some(bcc_email) = self.bcc_email {
            msg.push_str(&format!(
                "Bcc: {} <{}>\n",
                self.bcc_name.unwrap_or(""),
                bcc_email
            ));
        }

        if let Some(in_reply_to) = self.in_reply_to {
            msg.push_str(&format!("In-Reply-To: {}\n", in_reply_to));
        }

        if let Some(references) = self.references {
            msg.push_str(&format!("References: {}\n", references));
        }

        if let Some(reply_to) = self.reply_to {
            msg.push_str(&format!("Reply-To: {}\n", reply_to));
        }

        if let Some(content_type) = self.content_type {
            msg.push_str(&format!("Content-Type: {}\n", content_type));
        }

        if let Some(x_original_to) = self.x_original_to {
            msg.push_str(&format!("X-Original-To: {}\n", x_original_to));
        }

        if let Some(x_mailer) = self.x_mailer {
            msg.push_str(&format!("X-Mailer: {}\n", x_mailer));
        }

        if let Some(x_priority) = self.x_priority {
            msg.push_str(&format!("X-Priority: {}\n", x_priority));
        }

        if let Some(x_msmail_priority) = self.x_msmail_priority {
            msg.push_str(&format!("X-MSMail-Priority: {}\n", x_msmail_priority));
        }

        if let Some(x_spam_status) = self.x_spam_status {
            msg.push_str(&format!("X-Spam-Status: {}\n", x_spam_status));
        }

        if let Some(x_spam_score) = self.x_spam_score {
            msg.push_str(&format!("X-Spam-Score: {}\n", x_spam_score));
        }

        if let Some(x_spam_flag) = self.x_spam_flag {
            msg.push_str(&format!("X-Spam-Flag: {}\n", x_spam_flag));
        }

        if let Some(x_spam_report) = self.x_spam_report {
            msg.push_str(&format!("X-Spam-Report: {}\n", x_spam_report));
        }

        if let Some(x_spam_level) = self.x_spam_level {
            msg.push_str(&format!("X-Spam-Level: {}\n", x_spam_level));
        }

        if let Some(x_spam_checker_version) = self.x_spam_checker_version {
            msg.push_str(&format!(
                "X-Spam-Checker-Version: {}\n",
                x_spam_checker_version
            ));
        }

        msg.push('\n');
        msg.push_str(self.message);

        write!(f, "{}", msg)
    }
}

impl TryFrom<RawEmailMessage<'_>> for EmailMessage {
    type Error = crate::Error;

    fn try_from(value: RawEmailMessage) -> Result<Self> {
        EmailMessage::builder()
            .from(value.from_email)
            .to(value.to_email)
            .raw(&value.to_string())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_email_message() {
        let from_email = "from@example.com";
        let from_name = "From Name";
        let to_email = "to@example.com";
        let to_name = "To Name";
        let subject = "Test Email";
        let date = "Sat, 15 Mar 2025 22:06:02 +0000";
        let message_id = "<message_id>";
        let message = "Hello, World!";

        let msg = RawEmailMessage::builder()
            .from_email(from_email)
            .from_name(from_name)
            .to_email(to_email)
            .to_name(to_name)
            .subject(subject)
            .date(date)
            .message_id(message_id)
            .message(message)
            .build();

        let expected = format!(
            "From: {} <{}>\nTo: {} <{}>\nSubject: {}\nDate: {}\nMessage-ID: {}\n\n{}",
            from_name, from_email, to_name, to_email, subject, date, message_id, message
        );

        assert_eq!(msg.to_string(), expected);
    }
}
