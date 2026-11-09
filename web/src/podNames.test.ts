import { beforeEach, describe, expect, it } from 'vitest';
import { podHeading, rememberPod } from './podNames';

beforeEach(() => { try { sessionStorage.clear(); } catch { /* none */ } });

describe('the pod heading', () => {
  it('is the pod name once the client has seen it', () => {
    rememberPod({ id: 'pod_1', name: 'Launch room', createdAt: '2026-11-08T10:00:00Z' });
    expect(podHeading('pod_1')).toBe('Launch room');
  });

  it('falls back to the id while the name is unknown', () => {
    expect(podHeading('pod_unseen')).toBe('Pod pod_unseen');
  });

  it('is kept for the tab session, so a reload still shows the name', () => {
    rememberPod({ id: 'pod_2', name: 'Kept', createdAt: '2026-11-08T10:00:00Z' });
    expect(sessionStorage.getItem('jm.podNames')).toContain('Kept');
  });
});
