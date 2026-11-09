-- The sign-in hardening (ADR 0008, "Hardening"). A user token's expiry; the start limit's counts; which pods a token has
-- sockets in, for logout. No backfill: a user token minted before this keeps a null expires_at, which the Worker reads as
-- expired (computer and run tokens keep null, which never expires), so its holder signs in again.
alter table tokens add column expires_at real;
create table if not exists rate_limits (k text primary key, win integer not null, n integer not null);
create table if not exists token_pods (hash text not null, pod_id text not null, primary key (hash, pod_id));
