# Backup Normalizer

Backup Normalizer thins a flat directory of timestamped backup files into progressively coarser history. It keeps every recent backup, then one per local calendar day, ISO week, and calendar month.

Files that do not exactly match the configured timestamp format are ignored. The newest matching backup is always retained, and a redundant file must have unchanged metadata across two scans before it can be deleted.

> [!NOTE]
> This project contains AI-generated code. See [AI_USAGE.md](AI_USAGE.md) for details.

## Run

```yaml
services:
  backup-normalizer:
    image: ghcr.io/fopwoc/backup-normalizer:latest
    restart: unless-stopped
    user: "1000:1000"
    environment:
      LOG_LEVEL: info
      TZ: Europe/Moscow
    command:
      - --path=/backups
      - --keep-all-for=12h
      - --keep-daily-for=14d
      - --keep-weekly-for=12w
      - --keep-monthly-for=12mo
      - --scan-every=1h
      - --min-age=2h
      - --apply
    volumes:
      - ./backups:/backups
    read_only: true
    security_opt:
      - no-new-privileges:true
```

The default filename format is `%Y-%m-%d-%H-%M-%S.zip`, matching automatic ServerUtilities backups such as `2026-09-02-12-30-00.zip`. A name such as `before-new-base.zip` does not match and is never touched.

Without `--apply`, Backup Normalizer runs in dry-run mode and logs what it would remove. Use `backup-normalizer --help` for every option and supported retention unit.

## GTNH and ServerUtilities

The homelab GTNH server lets ServerUtilities create the backups and delegates retention to Backup Normalizer. Its `serverutilities.cfg` uses:

```properties
backups {
    B:enable_backups=true
    D:backup_timer=0.5
    I:backups_to_keep=2147483647
    B:delete_custom_name_backups=false
}
```

- `backup_timer=0.5` creates an automatic backup every 30 minutes.
- `backups_to_keep=2147483647` effectively disables ServerUtilities' count-based cleanup so it cannot fight the normalizer's time-based policy.
- `delete_custom_name_backups=false` preserves manually named backups. Backup Normalizer independently ignores them because their names do not match its timestamp format.
- Both containers mount the same backup directory: GTNH at `/data/backups` and Backup Normalizer at `/backups`.

The example retention values are the GTNH policy: retain every backup for 12 hours, then daily backups for 14 days, weekly backups for 12 weeks, and monthly backups for 12 months. `TZ` controls the local calendar boundaries used for those daily, weekly, and monthly buckets.

## Retention semantics

For the example configuration:

- Every backup from the latest 12 hours is retained.
- The newest backup in each local day is retained until it is 14 days old.
- The newest backup in each local ISO week is retained until it is 12 weeks old.
- The newest backup in each local month is retained until it is 12 months old.
- Older matching backups are removed.

The horizons are measured from the present, not added together. Supported units are `h`, `d`, `w`, `mo`, and `y`.

## Development

```bash
make
make image
```
