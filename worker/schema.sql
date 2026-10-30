create table if not exists users (id text primary key, name text not null);
create table if not exists computers (id text primary key, user_id text not null, name text not null, os text not null, arch text not null);
create table if not exists tokens (hash text primary key, kind text not null, subject text not null, pod text, revoked_at real);
create index if not exists tokens_subject on tokens (subject);
create table if not exists pods (id text primary key, created_at real not null);
create table if not exists invites (code text primary key, pod text not null);
create table if not exists agent_pods (computer_id text not null, pod_id text not null, primary key (computer_id, pod_id));
create table if not exists kv (k text primary key, v text not null);
