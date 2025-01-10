module Demo.RefInput (refInputMintingPolicy) where

import LambdaBuffers.Demo.Plutus.Plutarch (EqDatum, RefInputRedeemer (RefInputRedeemer))
import LambdaBuffers.Runtime.Plutarch ()
import LambdaBuffers.Runtime.Plutarch.LamVal (pfromPlutusDataPTryFrom)
import Plutarch.LedgerApi.V2 qualified as V2
import Plutarch.Prelude (ClosedTerm, PBool (PFalse), PData, PEq ((#==)), PMaybe (PJust), POpaque, PUnit (PUnit), pcon, perror, pfield, pfind, pfromData, pif, plam, pletC, pmatch, pmatchC, popaque, ptraceInfo, unTermCont, (#), (:-->))

{- | `eqValidator dat rdmr ctx` is a simple minting policy allowing to mint when the the redeemer
has a reference input to an EqValidator UTxO with a given inline datum
-}
refInputMintingPolicy :: ClosedTerm (PData :--> V2.PScriptContext :--> POpaque)
refInputMintingPolicy = plam $ \redeemer ctx -> ptraceInfo "[RefMint]" $ unTermCont $ do
  RefInputRedeemer refEqDatum refTxOutRef <-
    pmatchC $ pfromData $ pfromPlutusDataPTryFrom @RefInputRedeemer # redeemer

  refInputs <- pletC $ pfield @"referenceInputs" # (pfield @"txInfo" # ctx)

  PJust refInput <-
    pmatchC $
      pfind
        # plam (\txInInfo -> (pfield @"outRef" # txInInfo) #== pfromData refTxOutRef)
        # pfromData refInputs

  let validates =
        pmatch
          (pfield @"datum" # (pfield @"resolved" # refInput))
          $ \case
            V2.POutputDatum datum -> unTermCont $ do
              V2.PDatum stored <- pmatchC $ pfield @"outputDatum" # datum
              storedEqDatum <- pletC $ pfromData $ pfromPlutusDataPTryFrom @EqDatum # stored
              pure $ pfromData refEqDatum #== storedEqDatum
            _ -> pcon PFalse

  pure $ pif validates (popaque (pcon PUnit)) (ptraceInfo "[RefMint] Validation failed" perror)
