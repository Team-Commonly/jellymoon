import { expect, test } from 'bun:test';
import { githubOf, safeNext } from '../src/auth';
import { startupRefusal } from '../src/config';

test('next is a same-origin path or /', () => {
  for (const ok of ['/', '/device?code=BBBB-BBBB', '/a/b#c']) expect(safeNext(ok)).toBe(ok);
  for (const no of [undefined, '', 'https://evil.example/', '//evil.example', '/\\evil.example', 'javascript:x', 'evil', '/a\\b', '/a\r\nSet-Cookie: x', `/${'a'.repeat(512)}`]) {
    expect(safeNext(no)).toBe('/');
  }
});

test('GitHub endpoints: the defaults, an override in the Enterprise layout, none without a client', () => {
  expect(githubOf({ GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's' })).toEqual({ web: 'https://github.com', api: 'https://api.github.com', id: 'i', secret: 's' });
  expect(githubOf({ GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's', GITHUB_URL: 'https://ghe.example/' })?.api).toBe('https://ghe.example/api/v3');
  expect(githubOf({ GITHUB_CLIENT_ID: 'i' })).toBeNull();
});

test('an http GitHub override is refused outside ENV=local', () => {
  expect(startupRefusal({ DATABASE_URL: 'postgres://x', GITHUB_URL: 'http://ghe.example' })?.includes('GITHUB_URL')).toBe(true);
  expect(startupRefusal({ DATABASE_URL: 'postgres://x', GITHUB_URL: 'https://ghe.example' })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', GITHUB_URL: 'http://127.0.0.1:1' })).toBe(undefined);
});
