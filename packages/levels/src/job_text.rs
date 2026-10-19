use serde::{Deserialize, Serialize};

use crate::{Level, Symptom};

/// Who brought a generated job in and how they describe the first bug.
///
/// The complaint is about the first test of the job. The text comes from
/// shared messages, see [`JobText::message`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobText {
    /// One of [`JobText::CUSTOMERS`].
    pub customer: String,
    pub symptom: Symptom,
    /// Which of the [`JobText::TAKES`] ways to describe the symptom, from 1.
    pub take: u32,
}

impl JobText {
    /// The attributes of [`JobText::CUSTOMER_MESSAGE`], one per local.
    pub const CUSTOMERS: [&str; 16] = [
        "blib", "snorv", "twee", "gorm", "yalla", "krenn", "pibbo", "murg", "zazz", "oonk",
        "vettle", "drubb", "quilla", "hosk", "nibbet", "throm",
    ];
    pub const CUSTOMER_MESSAGE: &str = "job-customer";
    /// The remark on a board with a stuck RAM bit.
    pub const FAULT_REMARK: &str = "sticky";
    /// Holds every remark of generated jobs.
    pub const REMARK_MESSAGE: &str = "job-remarks";
    /// The ways each complaint message has to describe its symptom.
    pub const TAKES: u32 = 3;

    /// The message and attribute that hold `attribute` of a job with this text.
    ///
    /// `woke` says if the first test presses no keys. The complaint then
    /// comes from a `job-complaint-wake-` message.
    pub fn message(&self, attribute: &str, woke: bool) -> (String, String) {
        match attribute {
            Level::CUSTOMER => (Self::CUSTOMER_MESSAGE.into(), self.customer.clone()),
            Level::COMPLAINT => {
                let wake = if woke { "wake-" } else { "" };
                (
                    format!("job-complaint-{wake}{}", self.symptom.key()),
                    format!("take-{}", self.take),
                )
            }
            remark => (Self::REMARK_MESSAGE.into(), remark.into()),
        }
    }

    /// The remark for a further bug with `symptom`.
    pub fn also(symptom: Symptom) -> String {
        format!("also-{}", symptom.key())
    }
}
