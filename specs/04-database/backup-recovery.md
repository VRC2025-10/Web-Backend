# Backup & Recovery

## Backup Strategy

### Daily Full Backup

```bash
# Run daily via cron on the Docker host
pg_dump -Fc -h localhost -U vrc_user -d vrc_db > /backups/vrc_$(date +%Y%m%d_%H%M%S).dump
```

- Format: Custom (`-Fc`) for efficient restore with `pg_restore`
- Retention: 7 daily backups, 4 weekly backups (oldest daily deleted)
- Storage: Local filesystem on Proxmox VM host + optional rsync to backup host on same network

### Continuous WAL Archiving (Optional Enhancement)

For RPO < 1 hour, enable WAL archiving:

```ini
# postgresql.conf (Docker volume mount)
archive_mode = on
archive_command = 'cp %p /wal_archive/%f'
```

This enables Point-in-Time Recovery (PITR) to any transaction within the WAL retention window.

## Recovery Procedures

### Scenario 1: Application Crash (Database Intact)

```bash
docker compose restart app
```

Recovery time: ~30 seconds (container restart + migration check + health check).

### Scenario 2: Database Corruption or Data Loss

```bash
# Stop application
docker compose stop app

# Restore from latest backup
docker exec -i vrc-postgres pg_restore -c -d vrc_db < /backups/vrc_LATEST.dump

# Restart application (migrations will verify schema)
docker compose start app
```

Recovery time: ~5 minutes for a small database.

### Scenario 3: Complete Host Failure

1. Provision new Proxmox VM (or restore from VM snapshot)
2. Install Docker + docker-compose
3. Copy `.env` and `docker-compose.yml` to new host
4. Copy latest backup file to new host
5. `docker compose up -d db` — Start fresh PostgreSQL
6. `pg_restore` the backup
7. `docker compose up -d app` — Start application

Recovery time: ~15 minutes (VM clone or snapshot restore on Proxmox is fast).

## Verification

Monthly backup verification: restore latest backup to a separate Docker container and run `sqlx migrate info` to confirm schema integrity.
