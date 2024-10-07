// const npsUtils = require("nps-utils"); // not required, but handy!

module.exports = {
  scripts: {
    release: {
      default: "bash build_release.sh",
    },
    schema: {
      default:
        "nps schema.create   schema.asset-gauge schema.asset-staking schema.bribe-manager schema.connector-alliance schema.connector-emission schema.global-config schema.voting-escrow schema.zapper schema.phoenix-treasury",

      create: "bash scripts/build_schema.sh",

      "asset-gauge":
        "json2ts -i contracts/asset-gauge/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/asset-gauge",
      "asset-staking":
        "json2ts -i contracts/asset-staking/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/asset-staking",
      "bribe-manager":
        "json2ts -i contracts/bribe-manager/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/bribe-manager",
      "connector-alliance":
        "json2ts -i contracts/connector-alliance/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/connector-alliance",
      "connector-emission":
        "json2ts -i contracts/connector-emission/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/connector-emission",
      "global-config":
        "json2ts -i contracts/global-config/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/global-config",
      "voting-escrow":
        "json2ts -i contracts/voting-escrow/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/voting-escrow",
      zapper:
        "json2ts -i contracts/zapper/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/zapper",
      "phoenix-treasury":
        "json2ts -i contracts/phoenix-treasury/schema/raw/*.json -o ../liquid-staking-scripts/types/ve3/phoenix-treasury",
    },
  },
};
