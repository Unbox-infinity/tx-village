{ pkgs
, extraDDLs ? [ ]
, pgUser ? "tx_indexer"
, pgPort ? "5555"
, pgDir ? "./.pg"
, postgresql ? pkgs.postgresql_16
, extraPostgresConf ? ""
}:
let
  postgresConf =
    pkgs.writeText "postgresql.conf"
      ''
        # Add Custom Settings
        log_min_messages = warning
        log_min_error_statement = error
        log_min_duration_statement = 100  # ms
        log_connections = on
        log_disconnections = on
        log_duration = on
        #log_line_prefix = '[] '
        log_timezone = 'UTC'
        log_statement = 'all'
        log_directory = 'pg_log'
        log_filename = 'postgresql-%Y-%m-%d_%H%M%S.log'
        logging_collector = on
        log_min_error_statement = error

        ${extraPostgresConf}
      '';

  ddls = [ ./db/plutus.sql ] ++ extraDDLs;

  init-db = pkgs.writeShellApplication {
    name = "init-db";
    runtimeInputs = [ postgresql ];
    runtimeEnv = {
      LC_CTYPE = "en_US.UTF-8";
      LC_ALL = "en_US.UTF-8";
      LANG = "en_US.UTF-8";
      PGDATA = pgDir;
      PGHOST = pgDir;
    };
    text = ''
      [ ! -d "$PGDATA" ] && pg_ctl initdb -o "-U ${pgUser}" && cat "${postgresConf}" >> "$PGDATA/postgresql.conf"

      start-db
      echo "CREATE DATABASE ${pgUser}" | psql -p "${pgPort}" -U "${pgUser}" postgres

    '' + pkgs.lib.concatMapStringsSep "\n" (ddl: "pg < ${ddl}") ddls;
  };

  start-db = pkgs.writeShellApplication {
    name = "start-db";
    runtimeInputs = [ postgresql ];
    runtimeEnv = {
      PGDATA = pgDir;
    };
    text = ''
      pg_ctl -o "-p ${pgPort} -k $PGDATA" start
    '';
  };

  stop-db = pkgs.writeShellApplication {
    name = "stop-db";
    runtimeInputs = [ postgresql ];
    runtimeEnv = {
      PGDATA = pgDir;
    };
    text = ''
      pg_ctl stop && exit
    '';
  };

  pg = pkgs.writeShellApplication {
    name = "pg";
    runtimeInputs = [ postgresql ];
    runtimeEnv = {
      PGHOST = pgDir;
    };
    text = ''
      psql -p ${pgPort} -U ${pgUser}
    '';
  };

in
{
  inherit postgresConf;
  devShellTools = [ init-db start-db stop-db pg ];
}

