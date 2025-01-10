module Demo.Validation (eqValidator) where

import LambdaBuffers.Demo.Plutus.Plutarch (EqDatum, EqRedeemer (EqRedeemer'IsEqual, EqRedeemer'IsNotEqual))
import LambdaBuffers.Runtime.Plutarch ()
import LambdaBuffers.Runtime.Plutarch.LamVal (pfromPlutusDataPTryFrom)
import Plutarch.LedgerApi.V2 qualified as V2
import Plutarch.Prelude (ClosedTerm, PData, PEq ((#==)), POpaque, PUnit (PUnit), pcon, perror, pfromData, pif, plam, pletC, pmatch, pnot, popaque, pshow, ptraceC, ptraceInfo, unTermCont, (#), (:-->))

-- | `eqValidator dat rdmr ctx` checks whether the Datum `dat` is (not)equal to the Datum supplied in Redeemer.
eqValidator :: ClosedTerm (PData :--> PData :--> V2.PScriptContext :--> POpaque)
eqValidator = plam $ \datum redeemer _ctx -> ptraceInfo "[Mint]" $ unTermCont $ do
  eqDatum <- pletC $ pfromData $ pfromPlutusDataPTryFrom @EqDatum # datum
  ptraceC $ "[Mint] Datum is correct " <> pshow eqDatum
  eqRedeemer <- pletC $ pfromData $ pfromPlutusDataPTryFrom @EqRedeemer # redeemer
  ptraceC $ "[Mint] Redeemer is correct " <> pshow eqRedeemer

  validates <- pletC $ pmatch eqRedeemer $ \case
    EqRedeemer'IsEqual dat -> eqDatum #== pfromData dat
    EqRedeemer'IsNotEqual dat -> pnot # (eqDatum #== pfromData dat)

  pure $ pif validates (popaque (pcon PUnit)) (ptraceInfo "[Mint] Validation failed" perror)
