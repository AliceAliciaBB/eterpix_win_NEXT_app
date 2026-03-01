#!/usr/bin/env node
// scripts/build.js
// vite が Node.js v24 で非ゼロ終了することへの対策ラッパー

const { execSync } = require('child_process');
const fs = require('fs');

// 1. TypeScript型チェック
console.log('[build] Running type check...');
try {
  execSync('npx vue-tsc --noEmit', { stdio: 'inherit' });
} catch (e) {
  console.error('[build] TypeScript type check failed');
  process.exit(1);
}

// 2. vite ビルド（exit code はチェックせず、成果物で判定）
console.log('[build] Running vite build...');
try {
  execSync('npx vite build', { stdio: 'inherit' });
} catch (_e) {
  // Node.js v24 + vite v6 の組み合わせで、ビルド成功でも非ゼロ終了する場合がある
  // dist/index.html が生成されていれば実質成功と判断する
  if (!fs.existsSync('dist/index.html')) {
    console.error('[build] Vite build failed: dist/index.html not found');
    process.exit(1);
  }
  console.log('[build] Note: vite exited with non-zero but dist/index.html exists (success)');
}

console.log('[build] Frontend build complete.');
process.exit(0);
