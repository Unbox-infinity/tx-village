module Demo.Mint (mintingPolicy) where

import LambdaBuffers.Runtime.Plutarch ()
import Plutarch.LedgerApi.V2 qualified as V2
import Plutarch.Prelude (ClosedTerm, PAsData, PEq ((#==)), PInteger, POpaque, PUnit (PUnit), Term, pcon, perror, pfromData, pif, plam, popaque, ptraceInfo, unTermCont, (:-->))

{- | `eqValidator dat rdmr ctx` is a simple minting policy allowing to mint when the the redeemer matches the "secret"
Note: please don't get inspired by this implementation, anyone can see the secret
-}
mintingPolicy :: ClosedTerm (PAsData PInteger :--> (V2.PScriptContext :--> POpaque))
mintingPolicy = plam $ \redeemer _ctx -> ptraceInfo "[EQ]" $ unTermCont $ do
  let
    secret :: Term s PInteger
    secret = 1234

  let validates = pfromData redeemer #== secret

  pure $ pif validates (popaque (pcon PUnit)) (ptraceInfo "[EQ] Validation failed" perror)
