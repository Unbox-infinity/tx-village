{-# OPTIONS_GHC -Wno-orphans #-}

module Ledger.Sim.Types.CostModel (PlutusCostModel (..), mkEvaluationContext) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as M

import Data.Aeson.Types (
  FromJSON (parseJSON),
  FromJSONKey (fromJSONKey),
  FromJSONKeyFunction (FromJSONKeyTextParser),
  ToJSON (toJSON),
  ToJSONKey (toJSONKey),
  toJSONKeyText,
  withText,
 )

import Control.Monad.Except (runExcept)
import Control.Monad.Writer (WriterT (runWriterT))
import PlutusLedgerApi.Common qualified as Plutus
import PlutusLedgerApi.V2 qualified as PlutusV2

newtype PlutusCostModel = PlutusCostModel (Map PlutusV2.ParamName Integer)
  deriving newtype (ToJSON, FromJSON)

instance ToJSONKey PlutusV2.ParamName where
  toJSONKey = toJSONKeyText Plutus.showParamName

instance ToJSON PlutusV2.ParamName where
  toJSON = toJSON . Plutus.showParamName

instance FromJSONKey PlutusV2.ParamName where
  fromJSONKey = FromJSONKeyTextParser $ maybe (fail "Invalid Param Name") pure . Plutus.readParamName

instance FromJSON PlutusV2.ParamName where
  parseJSON = withText "ParamName" $ maybe (fail "Invalid Param Name") pure . Plutus.readParamName

mkEvaluationContext :: PlutusCostModel -> Either PlutusV2.CostModelApplyError PlutusV2.EvaluationContext
mkEvaluationContext (PlutusCostModel costModel) =
  runExcept
    . fmap fst
    . runWriterT
    $ PlutusV2.mkEvaluationContext (map fromIntegral (M.elems costModel))
