use std::fmt::Display;

use crate::colored::Colorize;
use cosmwasm_std::{attr, Attribute, StdError};
use cw_asset::Asset;
use cw_multi_test::AppResponse;
use ve3_shared::extensions::asset_ext::AssetExt;

pub trait EventChecker {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String;
  fn assert_attribute(&self, attr: Attribute) -> String;
  fn assert_transfer(&self, recipient: impl Into<String>, asset: Asset) -> String;
}

impl EventChecker for AppResponse {
  #[track_caller]
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String {
    let ty: String = ty.into();
    let found = self.events.iter().any(|a| {
      a.ty == ty && a.attributes.iter().any(|b| b.key == attr.key && b.value == attr.value)
    });

    if !found {
      let text = format!("Could not find attribute (\"{0}\", \"{1}\")", attr.key, attr.value);
      println!("{}", text.red());

      for event in self.events.iter() {
        let text = format!("{:?}", event);
        println!("{}", text.purple());
      }
      panic!("{:?}", StdError::generic_err(text));
    }

    attr.value
  }

  #[track_caller]
  fn assert_attribute(&self, attr: Attribute) -> String {
    self.assert_attribute_ty("wasm", attr)
  }

  #[track_caller]
  fn assert_transfer(&self, recipient: impl Into<String>, asset: Asset) -> String {
    match asset.info {
      cw_asset::AssetInfoBase::Native(_) => {
        self.assert_attribute_ty("transfer", attr("recipient", recipient));
        self.assert_attribute_ty("transfer", attr("amount", asset.to_coin().unwrap().to_string()))
      },
      cw_asset::AssetInfoBase::Cw20(_) => todo!(),
      _ => todo!(),
    }
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

  #[track_caller]
  fn assert_transfer(&self, recipient: impl Into<String>, asset: Asset) -> String {
    self.as_ref().unwrap().assert_transfer(recipient, asset)
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
