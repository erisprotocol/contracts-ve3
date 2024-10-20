use std::fmt::Display;

use crate::colored::Colorize;
use colored::Color;
use cosmwasm_std::{attr, Attribute, StdError};
use cw_asset::Asset;
use cw_multi_test::AppResponse;
use ve3_shared::extensions::asset_ext::AssetExt;

pub trait EventChecker {
  fn assert_attribute_ty(&self, ty: impl Into<String>, attr: Attribute) -> String;
  fn assert_attribute(&self, attr: Attribute) -> String;
  fn assert_transfer(&self, recipient: impl Into<String>, asset: Asset) -> String;
  fn get_attribute_value(&self, ty: impl Into<String>, key: String) -> String;
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
        let text = format!("{:?}:", event.ty);
        println!("{}", text.purple());

        for attr in event.attributes.iter() {
          let text = format!("res.assert_attribute(attr(\"{0}\", \"{1}\"));", attr.key, attr.value);

          if attr.key == "_contract_address" {
            println!(
              "{}",
              text.color(Color::TrueColor {
                r: 100,
                g: 100,
                b: 100
              })
            );
          } else if attr.key == "action" {
            println!("{}", text.yellow());
          } else {
            println!("{}", text.color(Color::Blue));
          }
        }
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
  fn get_attribute_value(&self, ty: impl Into<String>, key: String) -> String {
    let ty: String = ty.into();

    let found =
      self.events.iter().find(|a| a.ty == ty && a.attributes.iter().any(|b| b.key == key));

    match found {
      Some(a) => {
        let found = a.attributes.iter().find(|b| b.key == key).unwrap();
        found.value.to_string()
      },
      None => {
        let text = format!("Could not find attribute (\"{0}\")", key);
        println!("{}", text.red());

        for event in self.events.iter() {
          let text = format!("{:?}", event.ty);
          println!("{}", text.purple());

          for attr in event.attributes.iter() {
            let text = format!("\"{0}\",\"{1}\"", attr.key, attr.value);
            println!("{}", text.yellow());
          }
        }
        panic!("{:?}", StdError::generic_err(text));
      },
    }
  }

  #[track_caller]
  fn assert_transfer(&self, recipient: impl Into<String>, asset: Asset) -> String {
    match &asset.info {
      cw_asset::AssetInfoBase::Native(_) => {
        self.assert_attribute_ty("transfer", attr("recipient", recipient));
        self.assert_attribute_ty("transfer", attr("amount", asset.to_coin().unwrap().to_string()))
      },
      cw_asset::AssetInfoBase::Cw20(cw) => {
        self.assert_attribute(Attribute {
          key: "_contract_address".to_string(),
          value: cw.to_string(),
        });
        self.assert_attribute(attr("action", "transfer"));
        self.assert_attribute(attr("to", recipient));
        self.assert_attribute(attr("amount", asset.amount.to_string()))
      },
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
  fn get_attribute_value(&self, ty: impl Into<String>, key: String) -> String {
    self.as_ref().unwrap().get_attribute_value(ty, key)
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
