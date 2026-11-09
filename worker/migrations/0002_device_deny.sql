-- A device code a person refused in the browser (ADR 0008, step 4: POST /v1/auth/device/deny). Its poll gets access_denied,
-- and it can no longer be approved. Every code before this was not refused, so the default is the backfill.
alter table device_codes add column denied integer not null default 0;
