import Control.Exception (throwIO)
import Control.Monad.State (gets)
import Crypto.Hash (
  Blake2b_224 (Blake2b_224),
  Blake2b_256 (Blake2b_256),
  HashAlgorithm,
  hashWith,
 )
import Data.Bifunctor (first)
import Data.Binary qualified as B
import Data.ByteArray qualified as BA
import Data.ByteString (ByteString)
import Data.ByteString qualified as BS
import Data.ByteString.Base16 qualified as Base16
import Data.ByteString.Char8 qualified as BS8
import Data.ByteString.Lazy qualified as LBS
import Data.ByteString.Short qualified as SBS
import Data.Functor (void)
import Data.Map.Strict qualified as M
import Data.String (fromString)
import Ledger.Sim.Actions (getCurrentSlot, lookupUTxO, submitTx, throwAppError)
import Ledger.Sim.Test (
  ledgerFailsBy,
  ledgerSucceeds,
  ledgerTestCase,
  ledgerTestGroup,
  testCostModel,
 )
import Ledger.Sim.Types.LedgerConfig (LedgerConfig, ScriptMode (ScriptMode'AsWitness), mkLedgerConfig)
import Ledger.Sim.Types.LedgerSim (LedgerSimError (LedgerSimError'Submission))
import Ledger.Sim.Types.LedgerState (
  LedgerState (ls'currentTime),
  ledgerStateWithUtxos,
 )
import Ledger.Sim.Types.Submission (
  SubmissionError (SubmissionError'Validation),
  SubmissionResult (SubmissionResult),
 )
import Ledger.Sim.Utils.Hashing (hashDatum, hashScriptV2)
import Ledger.Sim.Validation (
  InvalidTxInfoError (
    InvalidTxInfoError'Local,
    InvalidTxInfoError'Normality,
    InvalidTxInfoError'Stateful
  ),
 )
import Ledger.Sim.Validation.Local qualified as Local
import Ledger.Sim.Validation.Normality qualified as Normality
import Ledger.Sim.Validation.Stateful qualified as Stateful
import PlutusLedgerApi.Common.Versions (vasilPV)
import PlutusLedgerApi.V1.Address (scriptHashAddress)
import PlutusLedgerApi.V1.Interval qualified as IV
import PlutusLedgerApi.V1.Value qualified as Value
import PlutusLedgerApi.V2 (
  Address (Address),
  BuiltinByteString,
  Credential (PubKeyCredential, ScriptCredential),
  CurrencySymbol (CurrencySymbol),
  Datum (Datum),
  OutputDatum (NoOutputDatum, OutputDatum, OutputDatumHash),
  POSIXTime,
  POSIXTimeRange,
  PubKeyHash (PubKeyHash),
  Redeemer (Redeemer),
  ScriptHash (ScriptHash, getScriptHash),
  ScriptPurpose (Minting, Spending),
  StakingCredential (StakingHash),
  ToData (toBuiltinData),
  TokenName (TokenName),
  TxId (TxId),
  TxInInfo (TxInInfo, txInInfoOutRef, txInInfoResolved),
  TxInfo (TxInfo),
  TxOut (TxOut, txOutAddress, txOutDatum, txOutReferenceScript, txOutValue),
  TxOutRef (TxOutRef),
  deserialiseScript,
  toBuiltin,
 )
import PlutusLedgerApi.V2 qualified as PlutusV2
import PlutusTx.AssocMap qualified as AssocMap
import Test.Tasty (TestTree, defaultMain)

main :: IO ()
main = do
  script <- either error pure . first show . deserialiseScript vasilPV $ SBS.toShort alwaysSucceedsCbor
  let sh = hashScriptV2 script
  ledgerCfg <- either throwIO pure $ mkLedgerConfig (M.fromList [(sh, (ScriptMode'AsWitness, script))]) testCostModel Nothing ()
  defaultMain $ tests sh ledgerCfg

tests :: ScriptHash -> LedgerConfig () -> TestTree
tests dummyScriptHash ledgerCfg =
  ledgerTestGroup
    ledgerCfg
    (ledgerStateWithUtxos (liftA2 M.singleton txInInfoOutRef txInInfoResolved dummyInput) ())
    "Tests"
    [ ledgerTestCase "Simple Minting TX" $
        ledgerSucceeds @() $ do
          let cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
              mintVal =
                Value.assetClassValue
                  (Value.AssetClass (cs, fromString "A"))
                  1
          currentTime <- gets ls'currentTime
          void . submitTx $
            TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0 <> mintVal
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0 <> mintVal)
              mempty
              AssocMap.empty
              (intervalValidRange currentTime (currentTime + 1))
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Minting cs, Redeemer (toBuiltinData @Integer 1234))])
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Simple Spending TX" $
        ledgerSucceeds $ do
          let datum = Datum $ toBuiltinData ()
              datumHash = hashDatum . Datum $ toBuiltinData ()
          currentTime <- getCurrentSlot
          SubmissionResult txId _ _ <-
            submitTx $
              TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0
                    , txOutReferenceScript = Nothing
                    , txOutDatum = OutputDatumHash datumHash
                    , txOutAddress = scriptHashAddress dummyScriptHash
                    }
                , txInInfoResolved dummyInput
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0)
                mempty
                AssocMap.empty
                (intervalValidRange currentTime (currentTime + 1))
                [ownPubKeyHash]
                AssocMap.empty
                AssocMap.empty
                dummyTxId
          let newUTxORef = TxOutRef txId 0
              newDummyInputRef = TxOutRef txId 1
          currentTime' <- getCurrentSlot
          newUTxO <-
            lookupUTxO newUTxORef >>= \case
              Just x -> pure x
              Nothing -> throwAppError "Newly created utxo absent from ledger state"
          void . submitTx $
            TxInfo
              [ TxInInfo newUTxORef newUTxO
              , dummyInput
                  { txInInfoOutRef = newDummyInputRef
                  }
              ]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              (intervalValidRange currentTime (currentTime' + 1))
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Spending newUTxORef, Redeemer $ toBuiltinData ())])
              (AssocMap.unsafeFromList [(datumHash, datum)])
              dummyTxId
    , ledgerTestCase "Invalid Range"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Stateful
                        ( Stateful.InvalidTxInfoError'InvalidValidRange
                            (Stateful.InvalidValidRangeError'CurrentTimeOutOfRange _)
                          )
                      ]
                  ) -> True
              _ -> False
          )
        $ do
          currentTime <- gets ls'currentTime
          void . submitTx $
            TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              (intervalValidRange (currentTime + 42) (currentTime + 4242))
              [ownPubKeyHash]
              AssocMap.empty
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Non-existent Input" $
        let nonexistentInputRef = TxOutRef dummyTxId' 1
         in ledgerFailsBy @()
              ( \case
                  LedgerSimError'Submission
                    ( SubmissionError'Validation
                        [ InvalidTxInfoError'Stateful
                            ( Stateful.InvalidTxInfoError'InvalidInputs
                                ( Stateful.InvalidInputsError'InvalidInput
                                    0
                                    (Stateful.InvalidInputError'NotFoundOnLedger txOutRef)
                                  )
                              )
                          ]
                      ) -> nonexistentInputRef == txOutRef
                  _ -> False
              )
              $ void . submitTx
              $ TxInfo
                [ TxInInfo
                    (TxOutRef dummyTxId' 1)
                    $ TxOut
                      { txOutValue = Value.lovelaceValue 0
                      , txOutReferenceScript = Nothing
                      , txOutDatum = NoOutputDatum
                      , txOutAddress = ownAddress
                      }
                ]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0
                    , txOutReferenceScript = Nothing
                    , txOutDatum = NoOutputDatum
                    , txOutAddress = ownAddress
                    }
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                AssocMap.empty
                AssocMap.empty
                dummyTxId
    , ledgerTestCase "Missing Signature"
        $ ledgerFailsBy
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Normality
                        ( Normality.InvalidTxInfoError'InvalidSignatories
                            Normality.InvalidSignatoriesError'NoSignature
                          )
                      , InvalidTxInfoError'Local
                          ( Local.InvalidTxInfoError'InvalidInputs
                              ( Local.InvalidInputsError'UnspendableInput
                                  0
                                  (Local.UnspendableInputError'PubKeyInput'NotAuthorized pkh)
                                )
                            )
                      ]
                  ) -> pkh == ownPubKeyHash
              _ -> False
          )
        $ do
          let
            cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
            mintVal =
              Value.assetClassValue
                (Value.AssetClass (cs, fromString "A"))
                1
          SubmissionResult txId _ _ <-
            submitTx $
              TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0 <> mintVal
                    , txOutReferenceScript = Nothing
                    , txOutDatum = NoOutputDatum
                    , txOutAddress = ownAddress
                    }
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0 <> mintVal)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                (AssocMap.unsafeFromList [(Minting cs, Redeemer (toBuiltinData @Integer 1234))])
                AssocMap.empty
                dummyTxId
          let newUTxORef = TxOutRef txId 0
          newUTxO <-
            lookupUTxO newUTxORef >>= \case
              Just x -> pure x
              Nothing -> throwAppError "Newly created utxo absent from ledger state"
          void . submitTx $
            TxInfo
              [TxInInfo newUTxORef newUTxO]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0 <> mintVal
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              IV.always
              mempty
              AssocMap.empty
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Unbalanced TX"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Local
                        (Local.InvalidTxInfoError'NotBalanced _) -- TODO(chfanghr): Assert delta
                      ]
                  ) -> True
              _ -> False
          )
        $ let cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
              mintVal =
                Value.assetClassValue
                  (Value.AssetClass (cs, fromString "A"))
                  1
           in void . submitTx $
                TxInfo
                  [dummyInput]
                  mempty
                  [ TxOut
                      { txOutValue = Value.lovelaceValue 0 <> mintVal
                      , txOutReferenceScript = Nothing
                      , txOutDatum = NoOutputDatum
                      , txOutAddress = ownAddress
                      }
                  ]
                  (Value.lovelaceValue 0)
                  (Value.lovelaceValue 0)
                  mempty
                  AssocMap.empty
                  IV.always
                  [ownPubKeyHash]
                  AssocMap.empty
                  AssocMap.empty
                  dummyTxId
    , ledgerTestCase "Missing Input Spent" $
        let ref = TxOutRef dummyTxId' 1
         in ledgerFailsBy @()
              ( \case
                  LedgerSimError'Submission
                    ( SubmissionError'Validation
                        [ InvalidTxInfoError'Local
                            ( Local.InvalidTxInfoError'InvalidRedeemers
                                ( Local.InvalidRedeemers'ExcessEntries
                                    [Spending txOutRef]
                                  )
                              )
                          ]
                      ) -> txOutRef == ref
                  _ -> False
              )
              $ void . submitTx
              $ TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0
                    , txOutReferenceScript = Nothing
                    , txOutDatum = NoOutputDatum
                    , txOutAddress = ownAddress
                    }
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                (AssocMap.unsafeFromList [(Spending ref, Redeemer $ toBuiltinData ())])
                AssocMap.empty
                dummyTxId
    , ledgerTestCase "Missing Datum Witness" $
        let dummyDatum = Datum $ toBuiltinData ()
            dummyDatumHash = hashDatum dummyDatum
         in ledgerFailsBy
              ( \case
                  LedgerSimError'Submission
                    ( SubmissionError'Validation
                        [ InvalidTxInfoError'Local
                            ( Local.InvalidTxInfoError'InvalidInputs
                                ( Local.InvalidInputsError'UnspendableInput
                                    0
                                    (Local.UnspendableInputError'ScriptInput'DatumWitnessMissing datumHash)
                                  )
                              )
                          ]
                      ) -> datumHash == dummyDatumHash
                  _ -> False
              )
              $ do
                SubmissionResult txId _ _ <-
                  submitTx $
                    TxInfo
                      [dummyInput]
                      mempty
                      [ TxOut
                          { txOutValue = Value.lovelaceValue 0
                          , txOutReferenceScript = Nothing
                          , txOutDatum = OutputDatumHash dummyDatumHash
                          , txOutAddress = scriptHashAddress dummyScriptHash
                          }
                      , txInInfoResolved dummyInput
                      ]
                      (Value.lovelaceValue 0)
                      (Value.lovelaceValue 0)
                      mempty
                      AssocMap.empty
                      IV.always
                      [ownPubKeyHash]
                      AssocMap.empty
                      AssocMap.empty
                      dummyTxId
                let newUTxORef = TxOutRef txId 0
                    newDummyInputRef = TxOutRef txId 1
                newUTxO <-
                  lookupUTxO newUTxORef >>= \case
                    Just x -> pure x
                    Nothing -> throwAppError "Newly created utxo absent from ledger state"
                void . submitTx $
                  TxInfo
                    [ TxInInfo newUTxORef newUTxO
                    , dummyInput
                        { txInInfoOutRef = newDummyInputRef
                        }
                    ]
                    mempty
                    [ TxOut
                        { txOutValue = Value.lovelaceValue 0
                        , txOutReferenceScript = Nothing
                        , txOutDatum = NoOutputDatum
                        , txOutAddress = ownAddress
                        }
                    ]
                    (Value.lovelaceValue 0)
                    (Value.lovelaceValue 0)
                    mempty
                    AssocMap.empty
                    IV.always
                    [ownPubKeyHash]
                    (AssocMap.unsafeFromList [(Spending newUTxORef, Redeemer $ toBuiltinData ())])
                    AssocMap.empty
                    dummyTxId
    , ledgerTestCase "Missing Policy Invocation"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Local
                        ( Local.InvalidTxInfoError'InvalidRedeemers
                            (Local.InvalidRedeemers'MintingPolicyNotRan (CurrencySymbol scriptHash))
                          )
                      ]
                  ) -> getScriptHash dummyScriptHash == scriptHash
              _ -> False
          )
        $ do
          let cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
              mintVal =
                Value.assetClassValue
                  (Value.AssetClass (cs, fromString "A"))
                  1
          void . submitTx $
            TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0 <> mintVal
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0 <> mintVal)
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              AssocMap.empty
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Missing Validator Invocation"
        $ ledgerFailsBy
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Local
                        ( Local.InvalidTxInfoError'InvalidRedeemers
                            (Local.InvalidRedeemers'ValidatorNotRan scriptHash txOutRef)
                          )
                      ]
                  ) -> scriptHash == dummyScriptHash && txOutRef == TxOutRef dummyTxId 0
              _ -> False
          )
        $ do
          let datum = Datum $ toBuiltinData ()
              datumHash = hashDatum . Datum $ toBuiltinData ()
          SubmissionResult txId _ _ <-
            submitTx $
              TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0
                    , txOutReferenceScript = Nothing
                    , txOutDatum = OutputDatumHash datumHash
                    , txOutAddress = scriptHashAddress dummyScriptHash
                    }
                , txInInfoResolved dummyInput
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                AssocMap.empty
                AssocMap.empty
                dummyTxId
          let newUTxORef = TxOutRef txId 0
              newDummyInputRef = TxOutRef txId 1
          newUTxO <-
            lookupUTxO newUTxORef >>= \case
              Just x -> pure x
              Nothing -> throwAppError "Newly created utxo absent from ledger state"
          void . submitTx $
            TxInfo
              [ TxInInfo newUTxORef newUTxO
              , dummyInput
                  { txInInfoOutRef = newDummyInputRef
                  }
              ]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              AssocMap.empty
              (AssocMap.unsafeFromList [(datumHash, datum)])
              dummyTxId
    , ledgerTestCase "Missing Datum In Script Output"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Local
                        ( Local.InvalidTxInfoError'InvalidOutputs
                            ( Local.InvalidOutputsError'InvalidOutput
                                0
                                Local.InvalidOutputError'ScriptOutputNoDatum
                              )
                          )
                      ]
                  ) -> True
              _ -> False
          )
        $ do
          void . submitTx $
            TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = scriptHashAddress dummyScriptHash
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              AssocMap.empty
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Not Script Input"
        $ ledgerFailsBy
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Local
                        ( Local.InvalidTxInfoError'InvalidRedeemers
                            ( Local.InvalidRedeemers'ExcessEntries
                                [Spending txOutRef]
                              )
                          )
                      ]
                  ) -> txOutRef == TxOutRef dummyTxId 0
              _ -> False
          )
        $ do
          let
            cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
            mintVal =
              Value.assetClassValue
                (Value.AssetClass (cs, fromString "A"))
                1
          SubmissionResult txId _ _ <-
            submitTx $
              TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0 <> mintVal
                    , txOutReferenceScript = Nothing
                    , txOutDatum = NoOutputDatum
                    , txOutAddress = ownAddress
                    }
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0 <> mintVal)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                (AssocMap.unsafeFromList [(Minting cs, Redeemer (toBuiltinData @Integer 1234))])
                AssocMap.empty
                dummyTxId
          let newUTxORef = TxOutRef txId 0
          newUTxO <-
            lookupUTxO newUTxORef >>= \case
              Just x -> pure x
              Nothing -> throwAppError "Newly created utxo absent from ledger state"
          void . submitTx $
            TxInfo
              [TxInInfo newUTxORef newUTxO]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0 <> mintVal
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Spending newUTxORef, Redeemer $ toBuiltinData ())])
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Non-normal Mint"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Normality
                        ( Normality.InvalidTxInfoError'InvalidMint
                            ( Normality.InvalidMintError'InvalidValue
                                ( Normality.InvalidValue
                                    _
                                    Normality.InvalidValueKind'AdaMissing
                                  )
                              )
                          )
                      ]
                  ) -> True
              _ -> False
          )
        $ do
          let
            cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
            mintVal =
              Value.assetClassValue
                (Value.AssetClass (cs, fromString "A"))
                1
          void . submitTx $
            TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0 <> mintVal
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              mintVal
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Minting cs, Redeemer (toBuiltinData @Integer 1234))])
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Non-normal Output Value"
        $ ledgerFailsBy @()
          ( \case
              LedgerSimError'Submission
                ( SubmissionError'Validation
                    [ InvalidTxInfoError'Normality
                        ( Normality.InvalidTxInfoError'InvalidOutputs
                            ( Normality.InvalidOutputsError'InvalidTxOut
                                0
                                ( Normality.InvalidTxOutError'InvalidValue
                                    ( Normality.InvalidValue
                                        _
                                        Normality.InvalidValueKind'AdaMissing
                                      )
                                  )
                              )
                          )
                      ]
                  ) -> True
              _ -> False
          )
        $ let
            cs = Value.CurrencySymbol $ getScriptHash dummyScriptHash
            mintVal =
              Value.assetClassValue
                (Value.AssetClass (cs, fromString "A"))
                1
           in
            void . submitTx $
              TxInfo
                [dummyInput]
                mempty
                [ TxOut
                    { txOutValue = mintVal
                    , txOutReferenceScript = Nothing
                    , txOutDatum = NoOutputDatum
                    , txOutAddress = ownAddress
                    }
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0 <> mintVal)
                mempty
                AssocMap.empty
                IV.always
                [ownPubKeyHash]
                (AssocMap.unsafeFromList [(Minting cs, Redeemer (toBuiltinData @Integer 1234))])
                AssocMap.empty
                dummyTxId
    , ledgerTestCase "Invalid Currency Symbol/Token Name/Address" $
        let
          invalidCurrencySymbol = CurrencySymbol $ toBuiltin $ BS.replicate 1280 42
          invalidTokenName = TokenName $ toBuiltin $ BS.replicate 1281 42
          invalidPubKeyHash = PubKeyHash $ toBuiltin $ BS.replicate 1282 42
          invalidScriptHash = ScriptHash $ toBuiltin $ BS.replicate 1283 42
          invalidAddress =
            Address
              (PubKeyCredential invalidPubKeyHash)
              (Just $ StakingHash $ ScriptCredential invalidScriptHash)
          mintValue =
            Value.assetClassValue
              (Value.AssetClass (invalidCurrencySymbol, invalidTokenName))
              1
          outputValue = Value.lovelaceValue 0 <> mintValue
         in
          ledgerFailsBy @()
            ( \case
                LedgerSimError'Submission
                  ( SubmissionError'Validation
                      [ InvalidTxInfoError'Normality
                          ( Normality.InvalidTxInfoError'InvalidOutputs
                              ( Normality.InvalidOutputsError'InvalidTxOut
                                  0
                                  ( Normality.InvalidTxOutError'InvalidAddress
                                      ( Normality.InvalidAddressError'InvalidCredential
                                          ( Normality.InvalidCredentialError'InvalidPubKeyCredential
                                              ( Normality.InvalidPubKeyHashError'UnexpectedLength
                                                  _
                                                  (Normality.InvalidLedgerBytesError'UnexpectedLength 28 1282)
                                                )
                                            )
                                        )
                                    )
                                )
                            )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidOutputs
                                ( Normality.InvalidOutputsError'InvalidTxOut
                                    0
                                    ( Normality.InvalidTxOutError'InvalidAddress
                                        ( Normality.InvalidAddressError'InvalidStakingCredential
                                            ( Normality.InvalidStakingCredentialError'InvalidCredential
                                                ( Normality.InvalidCredentialError'InvalidScriptCredential
                                                    ( Normality.InvalidScriptHashError'UnexpectedLength
                                                        _
                                                        (Normality.InvalidLedgerBytesError'UnexpectedLength 28 1283)
                                                      )
                                                  )
                                              )
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidOutputs
                                ( Normality.InvalidOutputsError'InvalidTxOut
                                    0
                                    ( Normality.InvalidTxOutError'InvalidValue
                                        ( Normality.InvalidValue
                                            _
                                            ( Normality.InvalidValueKind'InvalidMap
                                                ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                    _
                                                    _
                                                    ( Normality.InvalidAssetError'InvalidCurrencySymbol
                                                        ( Normality.InvalidCurrencySymbolError'UnexpectedLength
                                                            _
                                                            (Normality.InvalidLedgerBytesError'UnexpectedLength 28 1280)
                                                          )
                                                      )
                                                  )
                                              )
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidOutputs
                                ( Normality.InvalidOutputsError'InvalidTxOut
                                    0
                                    ( Normality.InvalidTxOutError'InvalidValue
                                        ( Normality.InvalidValue
                                            _
                                            ( Normality.InvalidValueKind'InvalidMap
                                                ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                    _
                                                    _
                                                    ( Normality.InvalidAssetError'InvalidNativeToken
                                                        ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                            _
                                                            1
                                                            ( Normality.InvalidNativeTokenError'InvalidTokenName
                                                                ( Normality.InvalidTokenNameError'TooLong
                                                                    _
                                                                    (Normality.InvalidLedgerBytesError'TooLong 32 1281)
                                                                  )
                                                              )
                                                          )
                                                      )
                                                  )
                                              )
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidMint
                                ( Normality.InvalidMintError'InvalidValue
                                    ( Normality.InvalidValue
                                        _
                                        ( Normality.InvalidValueKind'InvalidMap
                                            ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                _
                                                _
                                                ( Normality.InvalidAssetError'InvalidCurrencySymbol
                                                    ( Normality.InvalidCurrencySymbolError'UnexpectedLength
                                                        _
                                                        (Normality.InvalidLedgerBytesError'UnexpectedLength 28 1280)
                                                      )
                                                  )
                                              )
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidMint
                                ( Normality.InvalidMintError'InvalidValue
                                    ( Normality.InvalidValue
                                        _
                                        ( Normality.InvalidValueKind'InvalidMap
                                            ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                _
                                                _
                                                ( Normality.InvalidAssetError'InvalidNativeToken
                                                    ( Normality.InvalidOrderedAssocMapError'InvalidKeyValuePair
                                                        _
                                                        1
                                                        ( Normality.InvalidNativeTokenError'InvalidTokenName
                                                            ( Normality.InvalidTokenNameError'TooLong
                                                                _
                                                                (Normality.InvalidLedgerBytesError'TooLong 32 1281)
                                                              )
                                                          )
                                                      )
                                                  )
                                              )
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Normality
                            ( Normality.InvalidTxInfoError'InvalidRedeemers
                                ( Normality.InvalidRedeemersError'InvalidScriptPurpose
                                    ( Normality.InvalidScriptPurposeError'Minting
                                        ( Normality.InvalidCurrencySymbolError'UnexpectedLength
                                            _
                                            (Normality.InvalidLedgerBytesError'UnexpectedLength 28 1280)
                                          )
                                      )
                                  )
                              )
                        , InvalidTxInfoError'Stateful
                            ( Stateful.InvalidTxInfoError'InvalidRedeemers
                                (Stateful.InvalidRedeemersError {})
                              )
                        ]
                    ) -> True
                _ -> False
            )
            $ void . submitTx
            $ TxInfo
              [dummyInput]
              mempty
              [ TxOut
                  { txOutValue = outputValue
                  , txOutReferenceScript = Nothing
                  , txOutDatum = OutputDatum $ Datum $ toBuiltinData ()
                  , txOutAddress = invalidAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0 <> mintValue)
              mempty
              AssocMap.empty
              IV.always
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Minting invalidCurrencySymbol, Redeemer (toBuiltinData @Integer 1234))])
              AssocMap.empty
              dummyTxId
    , ledgerTestCase "Non-disjoint Reference Inputs"
        $ ledgerFailsBy
          ( \case
              LedgerSimError'Submission (SubmissionError'Validation [InvalidTxInfoError'Local (Local.InvalidTxInfoError'InvalidInputs (Local.InvalidInputsError'UsedAsRefInput 0))]) -> True
              _ -> False
          )
        $ do
          let datum = Datum $ toBuiltinData ()
              datumHash = hashDatum . Datum $ toBuiltinData ()
          currentTime <- getCurrentSlot
          SubmissionResult txId _ _ <-
            submitTx $
              TxInfo
                [dummyInput]
                [dummyInput]
                [ TxOut
                    { txOutValue = Value.lovelaceValue 0
                    , txOutReferenceScript = Nothing
                    , txOutDatum = OutputDatumHash datumHash
                    , txOutAddress = scriptHashAddress dummyScriptHash
                    }
                , txInInfoResolved dummyInput
                ]
                (Value.lovelaceValue 0)
                (Value.lovelaceValue 0)
                mempty
                AssocMap.empty
                (intervalValidRange currentTime (currentTime + 1))
                [ownPubKeyHash]
                AssocMap.empty
                AssocMap.empty
                dummyTxId
          let newUTxORef = TxOutRef txId 0
              newDummyInputRef = TxOutRef txId 1
          currentTime' <- getCurrentSlot
          newUTxO <-
            lookupUTxO newUTxORef >>= \case
              Just x -> pure x
              Nothing -> throwAppError "Newly created utxo absent from ledger state"
          void . submitTx $
            TxInfo
              [ TxInInfo newUTxORef newUTxO
              , dummyInput
                  { txInInfoOutRef = newDummyInputRef
                  }
              ]
              mempty
              [ TxOut
                  { txOutValue = Value.lovelaceValue 0
                  , txOutReferenceScript = Nothing
                  , txOutDatum = NoOutputDatum
                  , txOutAddress = ownAddress
                  }
              ]
              (Value.lovelaceValue 0)
              (Value.lovelaceValue 0)
              mempty
              AssocMap.empty
              (intervalValidRange currentTime (currentTime' + 1))
              [ownPubKeyHash]
              (AssocMap.unsafeFromList [(Spending newUTxORef, Redeemer $ toBuiltinData ())])
              (AssocMap.unsafeFromList [(datumHash, datum)])
              dummyTxId
    ]

intervalValidRange :: POSIXTime -> POSIXTime -> POSIXTimeRange
intervalValidRange from to = IV.Interval (IV.lowerBound from) (IV.strictUpperBound to)

dummyInput :: TxInInfo
dummyInput =
  let txOut =
        TxOut
          { txOutValue = Value.lovelaceValue 0
          , txOutReferenceScript = Nothing
          , txOutDatum = NoOutputDatum
          , txOutAddress = ownAddress
          }
      outRef = TxOutRef (dummyTxIds !! 42) 69
   in TxInInfo outRef txOut

ownAddress :: Address
ownAddress = Address (PubKeyCredential ownPubKeyHash) Nothing

ownPubKeyHash :: PubKeyHash
ownPubKeyHash = head dummyPubKeyHashes

dummyTxId :: PlutusV2.TxId
dummyTxId = head dummyTxIds

dummyTxId' :: PlutusV2.TxId
dummyTxId' = dummyTxIds !! 1

mkDummyHashes :: (HashAlgorithm alg) => alg -> [BuiltinByteString]
mkDummyHashes alg =
  toBuiltin
    . BA.convert @_ @ByteString
    . hashWith alg
    . LBS.toStrict
    . B.encode @Integer
    <$> [0 ..]

dummyBlake2b_224Hashes :: [BuiltinByteString]
dummyBlake2b_224Hashes = mkDummyHashes Blake2b_224

dummyBlake2b_256Hashes :: [BuiltinByteString]
dummyBlake2b_256Hashes = mkDummyHashes Blake2b_256

dummyPubKeyHashes :: [PubKeyHash]
dummyPubKeyHashes = PubKeyHash <$> dummyBlake2b_224Hashes

dummyTxIds :: [TxId]
dummyTxIds = TxId <$> dummyBlake2b_256Hashes

-- | Minting Policy that always succeeds.
alwaysSucceedsCbor :: BS8.ByteString
alwaysSucceedsCbor = either error id . Base16.decode $ BS8.pack "4d01000033222220051200120011"
