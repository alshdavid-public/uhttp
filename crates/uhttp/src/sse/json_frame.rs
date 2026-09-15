use std::fmt::Debug;

use anyhow::bail;
use serde::Serialize;

#[derive(Debug)]
pub struct JsonFrame<T: Serialize + Debug = ()> {
  event: Option<String>,
  data: Option<T>,
}

impl JsonFrame<()> {
  pub fn builder() -> JsonFrameBuilder<()> {
    JsonFrameBuilder {
      event: None,
      data: None,
    }
  }
}

impl<T: Serialize + Debug> JsonFrame<T> {
  pub fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
    let mut frame = Vec::new();

    if let Some(event) = &self.event {
      frame.extend_from_slice(b"event: ");
      frame.extend_from_slice(event.as_bytes());
      frame.push(b'\n');
    }

    if let Some(data) = &self.data {
      frame.extend_from_slice(b"data: ");
      serde_json::to_writer(&mut frame, data)?;
      frame.push(b'\n');
    }

    frame.push(b'\n');

    Ok(frame)
  }
}

#[derive(Debug)]
pub struct JsonFrameBuilder<T: Serialize + Debug = ()> {
  event: Option<String>,
  data: Option<T>,
}

impl<T: Serialize + Debug> JsonFrameBuilder<T> {
  pub fn event(
    mut self,
    event: impl Into<String>,
  ) -> Self {
    self.event = Some(event.into());
    self
  }

  #[allow(dead_code)]
  pub fn data<D: Serialize + Debug>(
    self,
    data: D,
  ) -> JsonFrameBuilder<D> {
    JsonFrameBuilder {
      event: self.event,
      data: Some(data),
    }
  }

  pub fn build(self) -> anyhow::Result<JsonFrame<T>> {
    if self.event.is_none() && self.data.is_none() {
      bail!("json frame needs an event, data, or both");
    }

    if let Some(event) = &self.event
      && event.contains(['\n', '\r'])
    {
      bail!("json frame event must not contain a line break: {event:?}");
    }

    Ok(JsonFrame {
      event: self.event,
      data: self.data,
    })
  }

  #[allow(clippy::wrong_self_convention)]
  pub fn as_frame(self) -> anyhow::Result<Vec<u8>> {
    self.build()?.to_vec()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn frames_carry_an_event_line_and_a_data_line() -> anyhow::Result<()> {
    let frame = JsonFrame::builder()
      .event("message")
      .data(vec!["a", "b"])
      .as_frame()?;
    assert_eq!(
      String::from_utf8(frame)?,
      "event: message\ndata: [\"a\",\"b\"]\n\n"
    );
    Ok(())
  }

  #[test]
  fn frames_without_an_event_omit_the_event_line() -> anyhow::Result<()> {
    let frame = JsonFrame::builder().data(vec![1, 2]).as_frame()?;
    assert_eq!(String::from_utf8(frame)?, "data: [1,2]\n\n");
    Ok(())
  }

  #[test]
  fn frames_without_data_omit_the_data_line() -> anyhow::Result<()> {
    let frame = JsonFrame::builder().event("heartbeat").as_frame()?;
    assert_eq!(String::from_utf8(frame)?, "event: heartbeat\n\n");
    Ok(())
  }

  #[test]
  fn a_unit_payload_sends_a_null_data_line() -> anyhow::Result<()> {
    let frame = JsonFrame::builder()
      .event("heartbeat")
      .data(())
      .as_frame()?;
    assert_eq!(
      String::from_utf8(frame)?,
      "event: heartbeat\ndata: null\n\n"
    );
    Ok(())
  }

  #[test]
  fn newlines_in_data_are_escaped_rather_than_breaking_the_frame() -> anyhow::Result<()> {
    let frame = JsonFrame::builder()
      .event("message")
      .data("one\ntwo")
      .as_frame()?;
    assert_eq!(
      String::from_utf8(frame)?,
      "event: message\ndata: \"one\\ntwo\"\n\n"
    );
    Ok(())
  }

  #[test]
  fn an_empty_frame_is_an_error() {
    assert!(JsonFrame::builder().as_frame().is_err());
  }

  #[test]
  fn a_frame_whose_event_has_a_line_break_is_an_error() {
    assert!(
      JsonFrame::builder()
        .event("bad\ndata: injected")
        .as_frame()
        .is_err()
    );
  }
}
