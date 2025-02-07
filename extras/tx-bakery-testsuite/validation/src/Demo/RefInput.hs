module Demo.RefInput (refInputMintingPolicy) where

import LambdaBuffers.Demo.Plutus.Plutarch (EqDatum, RefInputRedeemer (RefInputRedeemer))
import LambdaBuffers.Runtime.Plutarch ()
import LambdaBuffers.Runtime.Plutarch.LamVal (pfromPlutusDataPTryFrom)
import Plutarch.LedgerApi.V1 (PRedeemer (PRedeemer))
import Plutarch.LedgerApi.V3 qualified as V3
import Plutarch.Prelude (ClosedTerm, PBool (PFalse), PEq ((#==)), PMaybe (PJust), PUnit (PUnit), pcon, perror, pfield, pfind, pfromData, pif, plam, pletC, pmatch, pmatchC, ptraceInfo, unTermCont, (#), (:-->))

{- | `eqValidator dat rdmr ctx` is a simple minting policy allowing to mint when the the redeemer
has a reference input to an EqValidator UTxO with a given inline datum
-}
refInputMintingPolicy :: ClosedTerm (V3.PScriptContext :--> PUnit)
refInputMintingPolicy = plam $ \ctx -> ptraceInfo "[RefMint]" $ unTermCont $ do
  refInputs <- pletC $ pfield @"referenceInputs" # (pfield @"txInfo" # ctx)
  PRedeemer redeemer <- pmatchC $ pfield @"redeemer" # ctx
  RefInputRedeemer refEqDatum refTxOutRef <-
    pmatchC $ pfromData $ pfromPlutusDataPTryFrom @RefInputRedeemer # redeemer

  PJust refInput <-
    pmatchC $
      pfind
        # plam (\txInInfo -> (pfield @"outRef" # txInInfo) #== pfromData refTxOutRef)
        # pfromData refInputs

  let validates =
        pmatch
          (pfield @"datum" # (pfield @"resolved" # refInput))
          $ \case
            V3.POutputDatum datum -> unTermCont $ do
              V3.PDatum stored <- pmatchC $ pfield @"outputDatum" # datum
              storedEqDatum <- pletC $ pfromData $ pfromPlutusDataPTryFrom @EqDatum # stored
              pure $ pfromData refEqDatum #== storedEqDatum
            _ -> pcon PFalse

  pure $ pif validates (pcon PUnit) (ptraceInfo "[RefMint] Validation failed" perror)
