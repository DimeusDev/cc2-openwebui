use crate::printer::state::{EventKind, PrinterEvent};

pub struct Payload {
    pub title: String,
    pub body: String,
    pub color: u32,
}

pub fn format_event(event: &PrinterEvent) -> Payload {
    match &event.kind {
        EventKind::PrintStarted => Payload {
            title: "Print Started".to_string(),
            body: event.description.clone(),
            color: 0x3498db,
        },
        EventKind::PrintFinished => Payload {
            title: "Print Finished".to_string(),
            body: event.description.clone(),
            color: 0x2ecc71,
        },
        EventKind::PrintPaused => Payload {
            title: "Print Paused".to_string(),
            body: event.description.clone(),
            color: 0xf39c12,
        },
        EventKind::FailureNotifyThreshold => Payload {
            title: "Failure Risk Detected".to_string(),
            body: event.description.clone(),
            color: 0xe67e22,
        },
        EventKind::FailurePauseThreshold => Payload {
            title: "Print Failure Confirmed".to_string(),
            body: event.description.clone(),
            color: 0xe74c3c,
        },
        EventKind::AutoPaused => Payload {
            title: "Print Auto-Paused".to_string(),
            body: event.description.clone(),
            color: 0xe74c3c,
        },
        _ => Payload {
            title: "CC2 Monitor".to_string(),
            body: event.description.clone(),
            color: 0x95a5a6,
        },
    }
}
