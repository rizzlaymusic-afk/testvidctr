const { chromium } = require('playwright');
const path = require('path');

(async () => {
    const browser = await chromium.launch({
        headless: true, args: [
            '--autoplay-policy=no-user-gesture-required',
            '--enable-experimental-web-platform-features'
        ]
    });
    const context = await browser.newContext({ permissions: ['microphone', 'camera'] });
    const page = await context.newPage();

    const logs = [];
    page.on('console', msg => {
        const t = msg.text();
        logs.push(t);
        console.log('PAGE LOG>', t);
    });

    const url = 'http://127.0.0.1:8080/';
    console.log('Navigating to', url);
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 });

    // Upload local sample file to the file input
    await page.waitForSelector('#file-input', { timeout: 30000 });
    const samplePath = path.resolve(__dirname, '..', '..', 'assets', 'sample', 'sample.webm');
    console.log('Setting file input to', samplePath);
    await page.setInputFiles('#file-input', samplePath);

    // Wait for video metadata (duration > 0)
    console.log('Waiting for video metadata...');
    await page.waitForFunction(() => {
        const v = document.getElementById('player');
        return v && v.duration && v.duration > 0;
    }, { timeout: 30000 });
    const duration = await page.evaluate(() => document.getElementById('player').duration);
    console.log('Video duration (s):', duration);

    // Adjust trim handles via keyboard
    console.log('Adjusting trim handles');
    await page.focus('.trim-handle.start');
    for (let i = 0; i < 5; i++) await page.keyboard.press('ArrowRight');
    await page.focus('.trim-handle.end');
    for (let i = 0; i < 5; i++) await page.keyboard.press('ArrowLeft');

    // Trigger export
    console.log('Clicking Export');
    await page.click('.export-btn');

    // Wait for encoder console logs (or error) as a proxy for export completion
    console.log('Waiting for encoder logs or error (up to 60s)');
    const result = await (async () => {
        const timeoutAt = Date.now() + 60000;
        while (Date.now() < timeoutAt) {
            // check for known success logs
            if (logs.some(l => l.includes('encoder: calling final progress 1.0') || l.includes('encoder: assembled blob size') || l.includes('encoder: recorder stopped'))) {
                return { status: 'done' };
            }
            // check for visible UI error
            const errText = await page.evaluate(() => document.querySelector('.error')?.textContent?.trim() || null);
            if (errText) return { status: 'error', error: errText };
            await new Promise(r => setTimeout(r, 500));
        }
        return { status: 'timeout' };
    })();

    console.log('SMOKE RESULT', result);
    await browser.close();
    if (result.status === 'done') process.exit(0);
    else process.exit(2);
})().catch(e => { console.error(e); process.exit(3); });
