use std::fmt::Display;

use crate::colored::Colorize;
use cosmwasm_std::{Attribute, StdError, StdResult};
use cw_multi_test::AppResponse;

pub trait EventChecker {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String;
  fn assert_attribute(&self, attr: Attribute) -> String;
}

impl EventChecker for AppResponse {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String {
    let ty: String = ty.into();
    let found = self.events.iter().any(|a| {
      a.ty == ty && a.attributes.iter().any(|b| b.key == attr.key && b.value == attr.value)
    });

    if !found {
      let text = format!("Could not find attribute (\"{0}\", \"{1}\")", attr.key, attr.value);
      println!("{}", text.red());
      println!("{:?}", self.events);
      panic!("{:?}", StdError::generic_err(text));
    }

    attr.value
  }

  fn assert_attribute(&self, attr: Attribute) -> String {
    self.assert_attribute_ty("wasm", attr)
  }
}

impl EventChecker for Result<AppResponse, anyhow::Error> {
  #[track_caller]
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String {
    self.as_ref().unwrap().assert_attribute_ty(ty, attr)
  }

  #[track_caller]
  fn assert_attribute(&self, attr: Attribute) -> String {
    self.as_ref().unwrap().assert_attribute(attr)
  }
}

pub trait Valid {
  fn assert_valid(self);
  // fn assert_error<T: Display + std::fmt::Debug + Send + Sync + 'static>(self, err: impl Fn(T));
  fn assert_error<T: Display + std::fmt::Debug + Send + PartialEq + Sync + 'static>(self, err: T);
}

impl Valid for Result<AppResponse, anyhow::Error> {
  #[track_caller]
  fn assert_valid(self) {
    self.unwrap();
  }

  #[track_caller]
  fn assert_error<T: Display + std::fmt::Debug + Send + Sync + PartialEq + 'static>(self, err: T) {
    let error = self.unwrap_err();
    let res = error.downcast::<T>().unwrap();
    assert_eq!(res, err)
  }
  // fn assert_error<T: Display + std::fmt::Debug + Send + Sync + 'static>(self, err: T) {
  //   let error = self.unwrap_err();
  //   let res = error.downcast::<T>().unwrap();
  //   assert_eq!(res, err)
  // }
}
