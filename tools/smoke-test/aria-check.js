const { chromium } = require('playwright');
const path = require('path');

(async () => {
    const browser = await chromium.launch({
        headless: true, args: [
            '--autoplay-policy=no-user-gesture-required'
        ]
    });
    const context = await browser.newContext();
    const page = await context.newPage();

    page.on('console', msg => console.log('PAGE LOG>', msg.text()));

    const url = 'http://127.0.0.1:8080/';
    console.log('Navigating to', url);
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 });

    await page.waitForSelector('#file-input', { timeout: 30000 });
    const samplePath = path.resolve(__dirname, '..', '..', 'assets', 'sample', 'sample.webm');
    console.log('Setting file input to', samplePath);
    await page.setInputFiles('#file-input', samplePath);

    console.log('Waiting for video metadata...');
    await page.waitForFunction(() => {
        const v = document.getElementById('player');
        return v && v.duration && v.duration > 0;
    }, { timeout: 30000 });

    // ARIA sanity checks
    console.log('Checking ARIA attributes');
    const role = await page.getAttribute('.trim-handle.start', 'role');
    if (role !== 'slider') {
        console.error('Trim handle start missing role:', role);
        process.exit(2);
    }
    const orient = await page.getAttribute('.trim-handle.start', 'aria-orientation');
    if (orient !== 'horizontal') {
        console.error('Trim handle start missing aria-orientation:', orient);
        process.exit(2);
    }
    const controls = await page.getAttribute('.trim-handle.start', 'aria-controls');
    if (!controls || !controls.includes('player')) {
        console.error('Trim handle start aria-controls not pointing to player:', controls);
        process.exit(2);
    }
    const exportLabel = await page.getAttribute('.export-btn', 'aria-label');
    if (exportLabel !== 'Export trimmed video') {
        console.error('Export button aria-label wrong:', exportLabel);
        process.exit(2);
    }

    console.log('ARIA checks passed');
    await browser.close();
    process.exit(0);
})().catch(e => { console.error(e); process.exit(3); });
