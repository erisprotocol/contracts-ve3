use crate::colored::Colorize;
use cosmwasm_std::{Attribute, StdError, StdResult};
use cw_multi_test::AppResponse;

pub trait EventChecker {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> StdResult<String>;
  fn assert_attribute(&self, attr: Attribute) -> StdResult<String>;
}

impl EventChecker for AppResponse {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> StdResult<String> {
    let ty: String = ty.into();
    let found = self.events.iter().any(|a| {
      a.ty == ty && a.attributes.iter().any(|b| b.key == attr.key && b.value == attr.value)
    });

    if !found {
      let text = format!("Could not find attribute (\"{0}\", \"{1}\")", attr.key, attr.value);
      println!("{}", text.red());
      println!("{:?}", self.events);
      return Err(StdError::generic_err(text));
    }

    Ok(attr.value)
  }

  fn assert_attribute(&self, attr: Attribute) -> StdResult<String> {
    self.assert_attribute_ty("wasm", attr)
  }
}

impl EventChecker for Result<AppResponse, anyhow::Error> {
  #[track_caller]
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> StdResult<String> {
    self.as_ref().unwrap().assert_attribute_ty(ty, attr)
  }

  #[track_caller]
  fn assert_attribute(&self, attr: Attribute) -> StdResult<String> {
    self.as_ref().unwrap().assert_attribute(attr)
  }
}
