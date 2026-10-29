import { describe, expect, it } from 'vitest';

import { RateLimiter } from '../../../src/core/metadata/rateLimiter.js';

describe('RateLimiter', () => {
  it('permite llamadas hasta el límite sin esperar', async () => {
    const limiter = new RateLimiter(3, 1000);

    const start = Date.now();
    await limiter.waitForSlot();
    await limiter.waitForSlot();
    await limiter.waitForSlot();
    const elapsed = Date.now() - start;

    // Tres llamadas inmediatas no deberían esperar
    expect(elapsed).toBeLessThan(100);
  });

  it('bloquea cuando se excede el límite y espera a que pase la ventana', async () => {
    const limiter = new RateLimiter(2, 100);

    await limiter.waitForSlot();
    await limiter.waitForSlot();

    const start = Date.now();
    await limiter.waitForSlot();
    const elapsed = Date.now() - start;

    // Debería esperar aproximadamente 100 ms (la ventana)
    expect(elapsed).toBeGreaterThanOrEqual(80);
    expect(elapsed).toBeLessThan(500);
  });

  it('libera cupos después de que expire la ventana', async () => {
    const limiter = new RateLimiter(1, 50);

    await limiter.waitForSlot();
    await new Promise((r) => setTimeout(r, 60));

    const start = Date.now();
    await limiter.waitForSlot();
    const elapsed = Date.now() - start;

    // Después de esperar más que la ventana, no debería bloquear
    expect(elapsed).toBeLessThan(100);
  });

  it('limite de 1 permite una llamada por ventana', async () => {
    const limiter = new RateLimiter(1, 100);

    await limiter.waitForSlot();

    const start = Date.now();
    await limiter.waitForSlot();
    const elapsed = Date.now() - start;

    expect(elapsed).toBeGreaterThanOrEqual(80);
  });
});
