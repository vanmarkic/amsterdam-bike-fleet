import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration optimized for WebGL/deck.gl rendering.
 * GPU acceleration flags help significantly with map visualization performance.
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env['CI'],
  retries: process.env['CI'] ? 2 : 0,
  workers: process.env['CI'] ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL: 'http://localhost:4200',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',

    // GPU acceleration for WebGL/deck.gl performance
    launchOptions: {
      args: [
        // Enable GPU hardware acceleration
        '--enable-gpu',
        '--enable-webgl',
        '--enable-webgl2',
        '--ignore-gpu-blocklist',

        // Use ANGLE for better WebGL compatibility
        '--use-gl=angle',
        '--use-angle=default',

        // Disable software rendering fallback
        '--disable-software-rasterizer',

        // Additional performance flags
        '--enable-accelerated-2d-canvas',
        '--enable-zero-copy',
        '--enable-gpu-rasterization',

        // Disable features that slow down rendering
        '--disable-background-timer-throttling',
        '--disable-backgrounding-occluded-windows',
        '--disable-renderer-backgrounding',
      ],
    },
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    // Firefox and WebKit have different WebGL implementations
    // Uncomment if you need cross-browser testing
    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },
    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },
  ],

  // Run Angular dev server before tests
  webServer: {
    command: 'npm run start',
    url: 'http://localhost:4200',
    reuseExistingServer: !process.env['CI'],
    timeout: 120000,
  },
});
