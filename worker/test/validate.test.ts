import { expect, test } from 'bun:test';
import { check } from '../src/validate';

const post = { summary: 'ok', kind: 'fyi', status: 'done' };

test('a body is checked against the generated schema, limits included', () => {
  expect(check('PostInput', post)).toBeNull();
  expect(check('PostInput', { ...post, body: 'a'.repeat(16384) })).toBeNull();
  expect(check('PostInput', { ...post, body: 'a'.repeat(16385) })?.includes('at most 16384')).toBe(true);
  expect(check('PostInput', { ...post, body: 5 })?.includes('must be a string')).toBe(true);
  expect(check('PostInput', { ...post, summary: 'a\nb' })?.includes('pattern')).toBe(true);
  expect(check('PostInput', { summary: 'ok', status: 'done' })?.includes('kind: is required')).toBe(true);
  expect(check('PostInput', { ...post, receipt: {} })?.includes('not a field')).toBe(true);
});

test('length counts characters, not UTF-16 units', () => {
  expect(check('PostInput', { ...post, summary: '😀'.repeat(120) })).toBeNull();
  expect(check('PostInput', { ...post, summary: '😀'.repeat(121) })?.includes('at most 120')).toBe(true);
});

test('a field the URL supplies is not repeated in the body', () => {
  expect(check('TaskUpdateInput', { status: 'done' }, ['taskId'])).toBeNull();
  expect(check('TaskUpdateInput', { status: 'done', taskId: 'tsk_x' }, ['taskId'])?.includes('not a field')).toBe(true);
  expect(check('TaskUpdateInput', { status: 'done' })?.includes('taskId: is required')).toBe(true);
});

test('a prototype key is not a definition or a field', () => {
  expect(() => check('constructor', {})).toThrow();
  expect(check('PostInput', { ...post, constructor: 1 })?.includes('not a field')).toBe(true);
});
