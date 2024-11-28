module Ledger.Sim.Validation.Normality (
  InvalidPOSIXTimeError (..),
  InvalidLedgerBytesError'TooLong (..),
  InvalidLedgerBytesError'UnexpectedLength (..),
  InvalidTokenNameError (..),
  InvalidCurrencySymbolError (..),
  InvalidPubKeyHashError (..),
  InvalidScriptHashError (..),
  InvalidDatumHashError (..),
  InvalidTxIdError (..),
  InvalidTxOutRef (..),
  InvalidScriptPurposeError (..),
  InvalidCredentialError (..),
  InvalidStakingCredentialError (..),
  InvalidAddressError (..),
  InvalidOrderedAssocMapError (..),
  InvalidValueError (..),
  InvalidValueKind (..),
  InvalidAssetError (..),
  InvalidAdaTokenError (..),
  InvalidNativeTokenError (..),
  InvalidTxOutError (..),
  InvalidTxInInfoError (..),
  InvalidInputListError (..),
  InvalidInputsError (..),
  InvalidReferenceInputsError (..),
  InvalidOutputsError (..),
  InvalidFeeError (..),
  InvalidMintError (..),
  InvalidValidRangeError (..),
  InvalidSignatoriesError (..),
  InvalidRedeemersError (..),
  InvalidDataError (..),
  InvalidTxInfoError (..),
  validateTxInfo,
) where

import Control.Monad ((>=>))
import Data.ByteString qualified as BS
import Data.Functor.Contravariant (Contravariant (contramap))
import Data.Functor.Contravariant.Divisible (Decidable (choose))
import Data.Map qualified as M
import Data.Maybe (isJust)
import Data.Set qualified as S
import Ledger.Sim.Validation.Validator (
  Validator,
  contramapAndMapErr,
  contramapAndMapErrWithSubject,
  mapErr,
  mapErrWithSubject,
  validateFail,
  validateFoldable,
  validateIf,
  validateListAndAnnotateErrWithIdx,
  validateOptional,
  validatePass,
  validateRoundtrip,
  validateWith,
 )
import PlutusLedgerApi.V2 (
  Address (addressCredential, addressStakingCredential),
  Credential (PubKeyCredential, ScriptCredential),
  CurrencySymbol (unCurrencySymbol),
  Datum,
  DatumHash (DatumHash),
  Extended (Finite),
  Interval (ivFrom, ivTo),
  LowerBound (LowerBound),
  OutputDatum (OutputDatumHash),
  POSIXTime,
  POSIXTimeRange,
  PubKeyHash (getPubKeyHash),
  Redeemer,
  ScriptHash (getScriptHash),
  ScriptPurpose (Minting, Spending),
  StakingCredential (StakingHash),
  TokenName (unTokenName),
  TxId (getTxId),
  TxInInfo (txInInfoOutRef, txInInfoResolved),
  TxInfo (
    txInfoData,
    txInfoFee,
    txInfoId,
    txInfoInputs,
    txInfoMint,
    txInfoOutputs,
    txInfoRedeemers,
    txInfoReferenceInputs,
    txInfoSignatories,
    txInfoValidRange
  ),
  TxOut (txOutAddress, txOutDatum, txOutReferenceScript, txOutValue),
  TxOutRef (txOutRefId, txOutRefIdx),
  UpperBound (UpperBound),
  Value (getValue),
  adaSymbol,
  adaToken,
 )
import PlutusTx.AssocMap qualified as AssocMap
import PlutusTx.Builtins qualified as PlutusTx

--------------------------------------------------------------------------------

newtype InvalidPOSIXTimeError = InvalidPOSIXTimeError'NegativeInteger POSIXTime
  deriving stock (Show, Eq)

validatePOSIXTime :: Validator InvalidPOSIXTimeError POSIXTime
validatePOSIXTime = validateIf (>= 0) InvalidPOSIXTimeError'NegativeInteger

--------------------------------------------------------------------------------

validateLedgerBytesLength ::
  (Int -> Bool) ->
  (Int -> err) ->
  Validator err PlutusTx.BuiltinByteString
validateLedgerBytesLength checkLength mkErr =
  contramap (BS.length . PlutusTx.fromBuiltin) $
    validateIf checkLength mkErr

data InvalidLedgerBytesError'TooLong
  = InvalidLedgerBytesError'TooLong Int Int
  deriving stock (Show, Eq)

validateLedgerBytesLengthAtMost ::
  Int ->
  Validator InvalidLedgerBytesError'TooLong PlutusTx.BuiltinByteString
validateLedgerBytesLengthAtMost limit =
  validateLedgerBytesLength
    (<= limit)
    (InvalidLedgerBytesError'TooLong limit)

data InvalidLedgerBytesError'UnexpectedLength
  = InvalidLedgerBytesError'UnexpectedLength Int Int
  deriving stock (Show, Eq)

validateLedgerBytesLengthExactly ::
  Int ->
  Validator InvalidLedgerBytesError'UnexpectedLength PlutusTx.BuiltinByteString
validateLedgerBytesLengthExactly expected =
  validateLedgerBytesLength
    (== expected)
    (InvalidLedgerBytesError'UnexpectedLength expected)

validateBlake2b224Hash ::
  Validator
    InvalidLedgerBytesError'UnexpectedLength
    PlutusTx.BuiltinByteString
validateBlake2b224Hash = validateLedgerBytesLengthExactly 28

validateBlake2b256Hash ::
  Validator
    InvalidLedgerBytesError'UnexpectedLength
    PlutusTx.BuiltinByteString
validateBlake2b256Hash = validateLedgerBytesLengthExactly 32

--------------------------------------------------------------------------------

data InvalidTokenNameError
  = InvalidTokenNameError'TooLong TokenName InvalidLedgerBytesError'TooLong
  deriving stock (Show, Eq)

validateTokenName :: Validator InvalidTokenNameError TokenName
validateTokenName =
  mapErrWithSubject InvalidTokenNameError'TooLong $
    contramap unTokenName $
      validateLedgerBytesLengthAtMost 32

--------------------------------------------------------------------------------

data InvalidCurrencySymbolError
  = InvalidCurrencySymbolError'UnexpectedLength CurrencySymbol InvalidLedgerBytesError'UnexpectedLength
  deriving stock (Show, Eq)

validateCurrencySymbol :: Validator InvalidCurrencySymbolError CurrencySymbol
validateCurrencySymbol =
  mapErrWithSubject InvalidCurrencySymbolError'UnexpectedLength $
    choose
      ( \cs ->
          if cs == adaSymbol then Left () else Right cs
      )
      validatePass
      (contramap unCurrencySymbol validateBlake2b224Hash)

--------------------------------------------------------------------------------

data InvalidPubKeyHashError
  = InvalidPubKeyHashError'UnexpectedLength PubKeyHash InvalidLedgerBytesError'UnexpectedLength
  deriving stock (Show, Eq)

validatePubKeyHash :: Validator InvalidPubKeyHashError PubKeyHash
validatePubKeyHash =
  contramapAndMapErrWithSubject
    getPubKeyHash
    InvalidPubKeyHashError'UnexpectedLength
    validateBlake2b224Hash

--------------------------------------------------------------------------------

data InvalidScriptHashError
  = InvalidScriptHashError'UnexpectedLength ScriptHash InvalidLedgerBytesError'UnexpectedLength
  deriving stock (Show, Eq)

validateScriptHash :: Validator InvalidScriptHashError ScriptHash
validateScriptHash =
  contramapAndMapErrWithSubject
    getScriptHash
    InvalidScriptHashError'UnexpectedLength
    validateBlake2b224Hash

--------------------------------------------------------------------------------

data InvalidDatumHashError
  = InvalidDatumHashError'UnexpectedLength DatumHash InvalidLedgerBytesError'UnexpectedLength
  deriving stock (Show, Eq)

validateDatumHash :: Validator InvalidDatumHashError DatumHash
validateDatumHash =
  contramapAndMapErrWithSubject
    (\(DatumHash bs) -> bs)
    InvalidDatumHashError'UnexpectedLength
    validateBlake2b256Hash

--------------------------------------------------------------------------------

data InvalidTxIdError = InvalidTxIdError'UnexpectedLength TxId InvalidLedgerBytesError'UnexpectedLength
  deriving stock (Show, Eq)

validateTxId :: Validator InvalidTxIdError TxId
validateTxId =
  contramapAndMapErrWithSubject
    getTxId
    InvalidTxIdError'UnexpectedLength
    validateBlake2b256Hash

--------------------------------------------------------------------------------

data InvalidTxOutRef
  = InvalidTxOutRef'InvalidId InvalidTxIdError
  | InvalidTxOutRef'NegativeIndex Integer
  deriving stock (Show, Eq)

validateTxOutRef :: Validator InvalidTxOutRef TxOutRef
validateTxOutRef =
  mconcat
    [ contramapAndMapErr txOutRefId InvalidTxOutRef'InvalidId validateTxId
    , contramap txOutRefIdx $ validateIf (>= 0) InvalidTxOutRef'NegativeIndex
    ]

--------------------------------------------------------------------------------

data InvalidScriptPurposeError
  = InvalidScriptPurposeError'Minting InvalidCurrencySymbolError
  | InvalidScriptPurposeError'Spending InvalidTxOutRef
  | InvalidScriptPurposeError'Unsupported ScriptPurpose
  deriving stock (Show, Eq)

validateScriptPurpose :: Validator InvalidScriptPurposeError ScriptPurpose
validateScriptPurpose =
  choose
    ( \case
        Minting cs -> Left cs
        s -> Right s
    )
    (mapErr InvalidScriptPurposeError'Minting validateCurrencySymbol)
    $ choose
      ( \case
          Spending o -> Left o
          s -> Right s
      )
      (mapErr InvalidScriptPurposeError'Spending validateTxOutRef)
      (validateWith $ validateFail . InvalidScriptPurposeError'Unsupported)

--------------------------------------------------------------------------------

data InvalidCredentialError
  = InvalidCredentialError'InvalidPubKeyCredential InvalidPubKeyHashError
  | InvalidCredentialError'InvalidScriptCredential InvalidScriptHashError
  deriving stock (Show, Eq)

validateCredential :: Validator InvalidCredentialError Credential
validateCredential =
  choose
    ( \case
        PubKeyCredential c -> Left c
        ScriptCredential c -> Right c
    )
    (mapErr InvalidCredentialError'InvalidPubKeyCredential validatePubKeyHash)
    (mapErr InvalidCredentialError'InvalidScriptCredential validateScriptHash)

--------------------------------------------------------------------------------

data InvalidStakingCredentialError
  = InvalidStakingCredentialError'InvalidCredential InvalidCredentialError
  | InvalidStakingCredentialError'Unsupported StakingCredential
  deriving stock (Show, Eq)

validateStakingCredential :: Validator InvalidStakingCredentialError StakingCredential
validateStakingCredential =
  choose
    ( \case
        StakingHash h -> Left h
        c -> Right c
    )
    (mapErr InvalidStakingCredentialError'InvalidCredential validateCredential)
    ( mapErrWithSubject
        (const . InvalidStakingCredentialError'Unsupported)
        (validateFail ())
    )

--------------------------------------------------------------------------------

data InvalidAddressError
  = InvalidAddressError'InvalidCredential InvalidCredentialError
  | InvalidAddressError'InvalidStakingCredential InvalidStakingCredentialError
  deriving stock (Show, Eq)

validateAddress :: Validator InvalidAddressError Address
validateAddress =
  mconcat
    [ contramapAndMapErr addressCredential InvalidAddressError'InvalidCredential validateCredential
    , contramapAndMapErr addressStakingCredential InvalidAddressError'InvalidStakingCredential $
        validateOptional validateStakingCredential
    ]

--------------------------------------------------------------------------------

data InvalidOrderedAssocMapError k v e
  = InvalidOrderedAssocMapError'NotStrictlyOrdered [k] [k]
  | InvalidOrderedAssocMapError'InvalidKeyValuePair k v e
  deriving stock (Show, Eq)

validateOrderedAssocMap ::
  (Ord k) =>
  Validator e (k, v) ->
  Validator (InvalidOrderedAssocMapError k v e) (AssocMap.Map k v)
validateOrderedAssocMap validateEach =
  mconcat
    [ contramap AssocMap.keys $
        validateRoundtrip (S.toAscList . S.fromList) InvalidOrderedAssocMapError'NotStrictlyOrdered
    , contramap AssocMap.toList $
        validateFoldable $
          mapErrWithSubject
            (uncurry InvalidOrderedAssocMapError'InvalidKeyValuePair)
            validateEach
    ]

--------------------------------------------------------------------------------

data InvalidValueError = InvalidValue Value InvalidValueKind
  deriving stock (Show, Eq)

data InvalidValueKind
  = InvalidValueKind'InvalidMap
      ( InvalidOrderedAssocMapError
          CurrencySymbol
          (AssocMap.Map TokenName Integer)
          InvalidAssetError
      )
  | InvalidValueKind'AdaMissing
  | InvalidValueKind'AdaOnly
  deriving stock (Show, Eq)

data InvalidAssetError
  = InvalidAssetError'InvalidCurrencySymbol InvalidCurrencySymbolError
  | InvalidAssetError'InvalidAda (InvalidOrderedAssocMapError TokenName Integer InvalidAdaTokenError)
  | InvalidAssetError'InvalidNativeToken (InvalidOrderedAssocMapError TokenName Integer InvalidNativeTokenError)
  deriving stock (Show, Eq)

data InvalidAdaTokenError
  = InvalidAdaTokenError'TokenNameNotEmpty TokenName
  | InvalidAdaTokenError'NegativeAmount Integer
  | InvalidAdaTokenError'NonZeroAmount Integer
  deriving stock (Show, Eq)

data InvalidNativeTokenError
  = InvalidNativeTokenError'InvalidTokenName InvalidTokenNameError
  | InvalidNativeTokenError'ZeroAmount
  | InvalidNativeTokenError'ZeroOrNegativeAmount Integer
  deriving stock (Show, Eq)

data ValuePurpose
  = ValuePurpose'Output
  | ValuePurpose'Mint
  | ValuePurpose'Fee
  | ValuePurpose'Other

validateValue ::
  ValuePurpose ->
  Validator InvalidValueError Value
validateValue purpose =
  mapErrWithSubject InvalidValue $
    contramap getValue $
      mconcat
        [ mapErr InvalidValueKind'InvalidMap $
            validateOrderedAssocMap $
              choose
                ( \(cs, tnMap) ->
                    if cs == adaSymbol
                      then Left tnMap
                      else Right (cs, tnMap)
                )
                (mapErr InvalidAssetError'InvalidAda $ validateAdaTokens purpose)
                ( mconcat
                    [ contramap fst $
                        mapErr InvalidAssetError'InvalidCurrencySymbol validateCurrencySymbol
                    , contramap snd $
                        mapErr InvalidAssetError'InvalidNativeToken $
                          validateNativeTokenMap purpose
                    ]
                )
        , mapErr (const InvalidValueKind'AdaMissing) $ case purpose of
            ValuePurpose'Other -> mempty
            _ ->
              validateIf
                (isJust . (AssocMap.lookup adaSymbol >=> AssocMap.lookup adaToken))
                $ const ()
        , mapErr (const InvalidValueKind'AdaOnly) $ case purpose of
            ValuePurpose'Fee ->
              validateIf (([adaSymbol] ==) . AssocMap.keys) $ const ()
            _ -> mempty
        ]

validateAdaTokens ::
  ValuePurpose ->
  Validator
    (InvalidOrderedAssocMapError TokenName Integer InvalidAdaTokenError)
    (AssocMap.Map TokenName Integer)
validateAdaTokens purpose =
  validateOrderedAssocMap $
    mconcat
      [ contramap fst $ validateIf (== adaToken) InvalidAdaTokenError'TokenNameNotEmpty
      , contramap snd $
          mconcat
            [ validateIf (>= 0) InvalidAdaTokenError'NegativeAmount
            , case purpose of
                ValuePurpose'Mint -> validateIf (== 0) InvalidAdaTokenError'NonZeroAmount
                _ -> mempty
            ]
      ]

validateNativeTokenMap ::
  ValuePurpose ->
  Validator
    (InvalidOrderedAssocMapError TokenName Integer InvalidNativeTokenError)
    (AssocMap.Map TokenName Integer)
validateNativeTokenMap purpose =
  validateOrderedAssocMap $
    mconcat
      [ contramapAndMapErr fst InvalidNativeTokenError'InvalidTokenName validateTokenName
      , contramap snd $ case purpose of
          ValuePurpose'Output -> validateIf (> 0) InvalidNativeTokenError'ZeroOrNegativeAmount
          _ -> validateIf (/= 0) $ const InvalidNativeTokenError'ZeroAmount
      ]

--------------------------------------------------------------------------------

data InvalidTxOutError
  = InvalidTxOutError'InvalidAddress InvalidAddressError
  | InvalidTxOutError'InvalidValue InvalidValueError
  | InvalidTxOutError'InvalidOutputDatumHash InvalidDatumHashError
  | InvalidTxOutError'InvalidReferenceScriptHash InvalidScriptHashError
  deriving stock (Show, Eq)

validateTxOut :: Validator InvalidTxOutError TxOut
validateTxOut =
  mconcat
    [ contramapAndMapErr txOutAddress InvalidTxOutError'InvalidAddress validateAddress
    , contramapAndMapErr txOutValue InvalidTxOutError'InvalidValue $
        validateValue ValuePurpose'Output
    , contramapAndMapErr txOutDatum InvalidTxOutError'InvalidOutputDatumHash validateOutputDatum
    , contramapAndMapErr txOutReferenceScript InvalidTxOutError'InvalidReferenceScriptHash $
        validateOptional validateScriptHash
    ]
  where
    validateOutputDatum :: Validator InvalidDatumHashError OutputDatum
    validateOutputDatum =
      contramap
        ( \case
            OutputDatumHash h -> Just h
            _ -> Nothing
        )
        $ validateOptional validateDatumHash

--------------------------------------------------------------------------------

data InvalidTxInInfoError
  = InvalidTxInInfoError'InvalidOutRef InvalidTxOutRef
  | InvalidTxInInfoError'InvalidResolved InvalidTxOutError
  deriving stock (Show, Eq)

validateTxInInfo :: Validator InvalidTxInInfoError TxInInfo
validateTxInInfo =
  mconcat
    [ contramapAndMapErr txInInfoOutRef InvalidTxInInfoError'InvalidOutRef validateTxOutRef
    , contramapAndMapErr txInInfoResolved InvalidTxInInfoError'InvalidResolved validateTxOut
    ]

--------------------------------------------------------------------------------

data InvalidInputListError
  = InvalidInputListError'NotStrictlyOrdered
      [TxInInfo] -- Expected
      [TxInInfo] -- Actual
  | InvalidInputListError'InvalidTxInInfo
      Int -- Index
      InvalidTxInInfoError
  deriving stock (Show, Eq)

validateInputList :: Validator InvalidInputListError [TxInInfo]
validateInputList =
  mconcat
    [ validateRoundtrip
        ( fmap snd
            . M.toAscList
            . M.fromList
            . fmap (liftA2 (,) txInInfoOutRef id)
        )
        InvalidInputListError'NotStrictlyOrdered
    , validateListAndAnnotateErrWithIdx InvalidInputListError'InvalidTxInInfo validateTxInInfo
    ]

--------------------------------------------------------------------------------

data InvalidInputsError
  = InvalidInputsError'InvalidInputList InvalidInputListError
  | InvalidInputsError'NoPubKeyInput
  | InvalidInputsError'NoInput
  deriving stock (Show, Eq)

validateInputs :: Validator InvalidInputsError [TxInInfo]
validateInputs =
  mconcat
    [ mapErr InvalidInputsError'InvalidInputList validateInputList
    , validateIf (not . null) $ const InvalidInputsError'NoInput
    , validateIf
        ( any
            ( \txInInfo -> case addressCredential $ txOutAddress $ txInInfoResolved txInInfo of
                PubKeyCredential _ -> True
                _ -> False
            )
        )
        $ const InvalidInputsError'NoPubKeyInput
    ]

--------------------------------------------------------------------------------

newtype InvalidReferenceInputsError
  = InvalidReferenceInputsError'InvalidInputList InvalidInputListError
  deriving stock (Show, Eq)

validateReferenceInputs :: Validator InvalidReferenceInputsError [TxInInfo]
validateReferenceInputs =
  mapErr InvalidReferenceInputsError'InvalidInputList validateInputList

--------------------------------------------------------------------------------

data InvalidOutputsError
  = InvalidOutputsError'InvalidTxOut
      Int -- Index
      InvalidTxOutError
  | InvalidOutputsError'NoOutput
  deriving stock (Show, Eq)

validateOutputs :: Validator InvalidOutputsError [TxOut]
validateOutputs =
  mconcat
    [ validateListAndAnnotateErrWithIdx InvalidOutputsError'InvalidTxOut validateTxOut
    , validateIf (not . null) $ const InvalidOutputsError'NoOutput
    ]

--------------------------------------------------------------------------------

newtype InvalidFeeError = InvalidFeeError'InvalidValue InvalidValueError
  deriving stock (Show, Eq)

validateFee :: Validator InvalidFeeError Value
validateFee = mapErr InvalidFeeError'InvalidValue $ validateValue ValuePurpose'Fee

--------------------------------------------------------------------------------

newtype InvalidMintError = InvalidMintError'InvalidValue InvalidValueError
  deriving stock (Show, Eq)

validateMint :: Validator InvalidMintError Value
validateMint = mapErr InvalidMintError'InvalidValue $ validateValue ValuePurpose'Mint

--------------------------------------------------------------------------------

data InvalidValidRangeError
  = InvalidValidRangeError'LowerBound'InvalidPosixTime InvalidPOSIXTimeError
  | InvalidValidRangeError'UpperBound'InvalidPosixTime InvalidPOSIXTimeError
  | InvalidValidRangeError'UpperBound'CannotBeInclusive
  deriving stock (Show, Eq)

validateValidRange :: Validator InvalidValidRangeError POSIXTimeRange
validateValidRange =
  mconcat
    [ mapErr InvalidValidRangeError'LowerBound'InvalidPosixTime $
        contramap
          ((\(LowerBound e _) -> e) . ivFrom)
          validateExtended
    , contramap ((\(UpperBound e c) -> (e, c)) . ivTo) $
        mconcat
          [ validateIf
              ( \case
                  (Finite _, True) -> False
                  _ -> True
              )
              $ const InvalidValidRangeError'UpperBound'CannotBeInclusive
          , mapErr InvalidValidRangeError'UpperBound'InvalidPosixTime $
              contramap fst validateExtended
          ]
    ]
  where
    validateExtended :: Validator InvalidPOSIXTimeError (Extended POSIXTime)
    validateExtended =
      contramap
        ( \case
            Finite p -> Just p
            _ -> Nothing
        )
        $ validateOptional validatePOSIXTime

--------------------------------------------------------------------------------

data InvalidSignatoriesError
  = InvalidSignatoriesError'NotStrictlyOrdered
      [PubKeyHash]
      [PubKeyHash]
  | InvalidSignatoriesError'InvalidPubKeyHash Int InvalidPubKeyHashError
  | InvalidSignatoriesError'NoSignature
  deriving stock (Show, Eq)

validateSignatories :: Validator InvalidSignatoriesError [PubKeyHash]
validateSignatories =
  mconcat
    [ validateRoundtrip (S.toAscList . S.fromList) InvalidSignatoriesError'NotStrictlyOrdered
    , validateListAndAnnotateErrWithIdx InvalidSignatoriesError'InvalidPubKeyHash validatePubKeyHash
    , validateIf (not . null) $ const InvalidSignatoriesError'NoSignature
    ]

--------------------------------------------------------------------------------

data InvalidRedeemersError
  = InvalidRedeemersError'InvalidScriptPurpose InvalidScriptPurposeError
  | InvalidRedeemersError'HasDuplicateEntries
      (AssocMap.Map ScriptPurpose Redeemer)
      (AssocMap.Map ScriptPurpose Redeemer)
  deriving stock (Show, Eq)

validateRedeemers :: Validator InvalidRedeemersError (AssocMap.Map ScriptPurpose Redeemer)
validateRedeemers =
  mconcat
    [ contramap AssocMap.toList $
        validateFoldable $
          contramap fst $
            mapErr InvalidRedeemersError'InvalidScriptPurpose validateScriptPurpose
    , validateRoundtrip
        (AssocMap.safeFromList . reverse . AssocMap.toList)
        InvalidRedeemersError'HasDuplicateEntries
    ]

--------------------------------------------------------------------------------

newtype InvalidDataError
  = InvalidDataError'InvalidMap (InvalidOrderedAssocMapError DatumHash Datum InvalidDatumHashError)
  deriving stock (Show, Eq)

validateData :: Validator InvalidDataError (AssocMap.Map DatumHash Datum)
validateData =
  mapErr InvalidDataError'InvalidMap $
    validateOrderedAssocMap $
      contramap fst validateDatumHash

--------------------------------------------------------------------------------

data InvalidTxInfoError
  = InvalidTxInfoError'InvalidInputs InvalidInputsError
  | InvalidTxInfoError'InvalidReferenceInputs InvalidReferenceInputsError
  | InvalidTxInfoError'InvalidOutputs InvalidOutputsError
  | InvalidTxInfoError'InvalidFee InvalidFeeError
  | InvalidTxInfoError'InvalidMint InvalidMintError
  | InvalidTxInfoError'InvalidValidRange InvalidValidRangeError
  | InvalidTxInfoError'InvalidSignatories InvalidSignatoriesError
  | InvalidTxInfoError'InvalidRedeemers InvalidRedeemersError
  | InvalidTxInfoError'InvalidData InvalidDataError
  | InvalidTxInfoError'InvalidId InvalidTxIdError
  deriving stock (Show, Eq)

validateTxInfo :: Validator InvalidTxInfoError TxInfo
validateTxInfo =
  mconcat
    [ contramapAndMapErr txInfoInputs InvalidTxInfoError'InvalidInputs validateInputs
    , contramapAndMapErr txInfoReferenceInputs InvalidTxInfoError'InvalidReferenceInputs validateReferenceInputs
    , contramapAndMapErr txInfoOutputs InvalidTxInfoError'InvalidOutputs validateOutputs
    , contramapAndMapErr txInfoFee InvalidTxInfoError'InvalidFee validateFee
    , contramapAndMapErr txInfoMint InvalidTxInfoError'InvalidMint validateMint
    , contramapAndMapErr txInfoSignatories InvalidTxInfoError'InvalidSignatories validateSignatories
    , contramapAndMapErr txInfoValidRange InvalidTxInfoError'InvalidValidRange validateValidRange
    , contramapAndMapErr txInfoRedeemers InvalidTxInfoError'InvalidRedeemers validateRedeemers
    , contramapAndMapErr txInfoData InvalidTxInfoError'InvalidData validateData
    , contramapAndMapErr txInfoId InvalidTxInfoError'InvalidId validateTxId
    ]
