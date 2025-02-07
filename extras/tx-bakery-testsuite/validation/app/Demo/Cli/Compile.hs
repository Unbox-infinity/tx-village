module Demo.Cli.Compile (CompileOpts (..), CompileMode (..), compile) where

import Data.ByteString qualified
import Data.ByteString.Short (fromShort)
import Demo.Mint (mintingPolicy)
import Demo.RefInput (refInputMintingPolicy)
import Demo.Validation (eqValidator)
import Demo.ValidationV2 (eqValidatorV2)
import LambdaBuffers.Demo.Config (
  Config (Config, config'eqValidator, config'eqValidatorV2, config'mintingPolicy, config'refInputMintingPolicy),
  Script (Script),
 )
import LambdaBuffers.Runtime.Prelude (toJsonBytes)
import Plutarch.Internal.Term qualified as Term (Config (NoTracing, Tracing), LogLevel (LogInfo), TracingMode (DoTracing), compile)
import Plutarch.Script (serialiseScript)

data CompileMode = COMPILE_PROD | COMPILE_DEBUG deriving stock (Show, Read, Eq)

data CompileOpts = CompileOpts
  { co'Mode :: CompileMode
  , co'File :: FilePath
  }
  deriving stock (Show, Eq)

compile :: CompileOpts -> IO ()
compile opts = do
  let cfg = case co'Mode opts of
        COMPILE_PROD -> Term.NoTracing
        COMPILE_DEBUG -> Term.Tracing Term.LogInfo Term.DoTracing

  eqValidatorCompiled <-
    either
      (\err -> fail $ "Failed compiling eqValidator with " <> show err)
      pure
      (Term.compile cfg eqValidator)

  eqValidatorV2Compiled <-
    either
      (\err -> fail $ "Failed compiling eqValidatorV2 with " <> show err)
      pure
      (Term.compile cfg eqValidatorV2)

  mintingPolicyCompiled <-
    either
      (\err -> fail $ "Failed compiling mintingPolicy with " <> show err)
      pure
      (Term.compile cfg mintingPolicy)

  refInputMintingPolicyCompiled <-
    either
      (\err -> fail $ "Failed compiling refInputMintingPolicy with " <> show err)
      pure
      (Term.compile cfg refInputMintingPolicy)

  let config =
        toJsonBytes $
          Config
            { config'eqValidator = Script (fromShort . serialiseScript $ eqValidatorCompiled)
            , config'eqValidatorV2 = Script (fromShort . serialiseScript $ eqValidatorV2Compiled)
            , config'mintingPolicy = Script (fromShort . serialiseScript $ mintingPolicyCompiled)
            , config'refInputMintingPolicy = Script (fromShort . serialiseScript $ refInputMintingPolicyCompiled)
            }
  Data.ByteString.writeFile (co'File opts) config
