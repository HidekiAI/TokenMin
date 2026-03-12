#!/usr/bin/env node

/**
 * TokenMin Gemini CLI Hook Installer
 * 
 * This script builds the TokenMin WASM module, creates a plugin wrapper,
 * and registers it with the Gemini CLI configuration.
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

const askQuestion = (query) => new Promise(resolve => rl.question(query, resolve));

async function main() {
    console.log('🪙  TokenMin WASM Hook Installer for Gemini CLI');
    
    // Step 1: Ensure WASM is built
    console.log('\n[1/4] Building TokenMin WASM module...');
    try {
        execSync('wasm-pack build --target nodejs', { stdio: 'inherit' });
    } catch (e) {
        console.error('Failed to build WASM. Do you have wasm-pack installed?');
        process.exit(1);
    }
    
    // Step 2: Locate Gemini config
    console.log('\n[2/4] Locating Gemini CLI configuration...');
    
    let isGeminiInstalled = false;
    try {
        execSync('which gemini', { stdio: 'ignore' });
        isGeminiInstalled = true;
    } catch (e) {
        console.warn('⚠️  Warning: `gemini` command not found in PATH. Are you sure Gemini CLI is installed?');
    }

    if (!isGeminiInstalled) {
        const proceedAnswer = await askQuestion('`gemini` was not found in your PATH. Continue installing the hook anyway? (y/N): ');
        const normalized = proceedAnswer.trim().toLowerCase();
        if (normalized !== 'y' && normalized !== 'yes') {
            console.log('Aborting installation because `gemini` is not installed or not in PATH.');
            rl.close();
            process.exit(1);
        }
    }
    const possiblePaths = [
        path.join(process.env.HOME || process.env.USERPROFILE, '.gemini', 'settings.json'),
        path.join(process.cwd(), '.gemini', 'settings.json') // Fallback to workspace settings
    ];
    
    let configPath = null;
    for (const p of possiblePaths) {
        if (fs.existsSync(p)) {
            configPath = p;
            break;
        }
    }
    
    if (!configPath) {
        console.log('Could not automatically locate settings.json.');
        console.log('We will create a new config file in ~/.gemini/settings.json if you press Enter, or you can specify a custom path.');
        const answer = await askQuestion('Path to config directory (Press Enter for ~/.gemini/): ');
        
        let targetDir;
        if (!answer.trim()) {
            targetDir = path.join(process.env.HOME || process.env.USERPROFILE, '.gemini');
        } else {
            targetDir = answer.trim();
            if (targetDir.endsWith('.json')) {
                targetDir = path.dirname(targetDir);
            }
        }

        if (!fs.existsSync(targetDir)) {
            fs.mkdirSync(targetDir, { recursive: true });
        }
        
        configPath = path.join(targetDir, 'settings.json');
        if (!fs.existsSync(configPath)) {
            console.log(`Creating new config file at: ${configPath}`);
            fs.writeFileSync(configPath, JSON.stringify({ hooks: {} }, null, 2));
        }
    }
    
    console.log(`Found config at: ${configPath}`);
    const configDir = path.dirname(configPath);
    
    // Step 3: Copy WASM and generate wrapper
    console.log('\n[3/4] Installing hook files...');
    
    const tokenminDir = path.join(configDir, 'tokenmin');
    if (!fs.existsSync(tokenminDir)) {
        fs.mkdirSync(tokenminDir, { recursive: true });
    }
    
    // Copy WASM build artifacts
    const pkgDir = path.join(process.cwd(), 'pkg');
    if (!fs.existsSync(pkgDir)) {
         console.error('WASM build directory (pkg/) not found!');
         process.exit(1);
    }
    
    // Copy the WASM files over
    fs.cpSync(pkgDir, path.join(tokenminDir, 'pkg'), { recursive: true });
    
    // Generate Wrapper Plugin (Node script that calls WASM)
    const pluginPath = path.join(tokenminDir, 'tokenmin-hook.js');
    const wrapperCode = `
// TokenMin Gemini CLI Hook
const { TokenMinWasm } = require('./pkg/token_min.js');

const bypassModels = process.env.TOKENMIN_BYPASS_MODELS ? process.env.TOKENMIN_BYPASS_MODELS.split(',') : ['copilot-chat', 'gpt-3.5-turbo'];
const summarizerUrl = process.env.OLLAMA_URL || 'http://localhost:11434';
const summarizerModel = process.env.TOKENMIN_SUMMARIZER_MODEL || 'qwen2.5-coder';

const tokenmin = new TokenMinWasm(bypassModels, summarizerUrl, summarizerModel);

// Hook input is provided as a JSON string via process.env.GEMINI_CLI_HOOK_INPUT
const fs = require('fs');

// Gemini CLI passes the JSON payload via stdin
let rawInput = '';
try {
    rawInput = fs.readFileSync(0, 'utf-8');
} catch (e) {
    // ignore
}

if (!rawInput) {
    rawInput = process.env.GEMINI_CLI_HOOK_INPUT || process.env.CLAUDE_CODE_HOOK_INPUT || process.argv[2];
}

if (!rawInput) {
    console.error('[TokenMin] No input provided to hook.');
    process.exit(1);
}
const input = JSON.parse(rawInput);

async function run() {
    try {
        const rawContent = JSON.stringify(input.llm_request.contents);
        const model = input.llm_request.model;
        
        const compressedContent = await tokenmin.compress(rawContent, model);
        
        console.log(JSON.stringify({
            hookSpecificOutput: {
                hookEventName: 'BeforeModel',
                llm_request: {
                    contents: JSON.parse(compressedContent)
                }
            }
        }));
    } catch (err) {
        console.error('[TokenMin] Compression failed:', err);
        // On failure, don't modify the request
        console.log(JSON.stringify({}));
    }
}

run();
`;
    fs.writeFileSync(pluginPath, wrapperCode.trim());
    console.log(`Hook wrapper installed to: ${pluginPath}`);
    
    // Step 4: Register with config
    console.log('\n[4/4] Registering hook in Gemini CLI settings.json...');
    try {
        const configData = JSON.parse(fs.readFileSync(configPath, 'utf8'));
        
        if (!configData.hooks) {
            configData.hooks = {};
        }
        if (!configData.hooks.BeforeModel) {
            configData.hooks.BeforeModel = [];
        }
        
        const hookCommand = `node "${pluginPath.replace(/\\\\/g, '/')}"`;
        
        // Check if already registered
        const alreadyRegistered = configData.hooks.BeforeModel.some(
            entry => entry.hooks && entry.hooks.some(h => h.name === 'tokenmin-compress')
        );
        
        if (!alreadyRegistered) {
            configData.hooks.BeforeModel.push({
                matcher: "*",
                hooks: [
                    {
                        name: "tokenmin-compress",
                        type: "command",
                        command: hookCommand
                    }
                ]
            });
            fs.writeFileSync(configPath, JSON.stringify(configData, null, 2));
            console.log('Successfully registered TokenMin in settings.json');
        } else {
            console.log('TokenMin is already registered in the configuration.');
        }
        
    } catch (e) {
        console.error('Failed to parse or update settings.json:', e);
        console.log('Please manually add the TokenMin hook to your Gemini CLI settings.json. Under "hooks.BeforeModel", add an entry like:');
        console.log(`  {`);
        console.log(`    "matcher": "*",`);
        console.log(`    "hooks": [`);
        console.log(`      {`);
        console.log(`        "name": "tokenmin-compress",`);
        console.log(`        "type": "command",`);
        console.log(`        "command": "node \\"${pluginPath.replace(/\\\\/g, '/')}\\\""`);
        console.log(`      }`);
        console.log(`    ]`);
        console.log(`  }`);
    }
    
    console.log('\n✨ TokenMin WASM installation complete!');
    rl.close();
}

main().catch(console.error);
