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
    
    const possiblePaths = [
        path.join(process.env.HOME || process.env.USERPROFILE, '.gemini', 'gemini.config.json'),
        path.join(process.env.HOME || process.env.USERPROFILE, '.config', 'gemini', 'gemini.config.json'),
        path.join(process.cwd(), 'gemini.config.json') // Fallback to cwd
    ];
    
    let configPath = null;
    for (const p of possiblePaths) {
        if (fs.existsSync(p)) {
            configPath = p;
            break;
        }
    }
    
    if (!configPath) {
        console.log('Could not automatically locate gemini.config.json.');
        const answer = await askQuestion('Please enter the full path to your gemini.config.json directory: ');
        const testPath = path.join(answer.trim(), 'gemini.config.json');
        if (fs.existsSync(testPath)) {
            configPath = testPath;
        } else if (fs.existsSync(answer.trim()) && answer.trim().endsWith('.json')) {
            configPath = answer.trim();
        } else {
            console.error('File still not found. Please create the config file first or provide the correct path.');
            process.exit(1);
        }
    }
    
    console.log(`Found config at: ${configPath}`);
    const configDir = path.dirname(configPath);
    
    // Step 3: Copy WASM and generate wrapper
    console.log('\n[3/4] Installing plugin files...');
    
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
    
    // Generate Wrapper Plugin
    const pluginPath = path.join(tokenminDir, 'tokenmin-plugin.js');
    const wrapperCode = `
// TokenMin Gemini CLI Extension Wrapper
const { TokenMinWasm } = require('./pkg/token_min.js');

let tokenmin = null;

function initTokenMin(config) {
    if (!tokenmin) {
        const bypassModels = config.tokenmin?.bypassModels || ['copilot-chat', 'gpt-3.5-turbo'];
        const summarizerUrl = config.tokenmin?.summarizerUrl || process.env.OLLAMA_URL || 'http://localhost:11434';
        const summarizerModel = config.tokenmin?.summarizerModel || 'qwen2.5-coder';
        
        tokenmin = new TokenMinWasm(bypassModels, summarizerUrl, summarizerModel);
    }
    return tokenmin;
}

module.exports = {
    name: 'TokenMin',
    version: '1.0.0',
    
    // Register for the BeforeModel hook
    hooks: {
        async beforeModel(event) {
            const { model, config, contents } = event;
            const engine = initTokenMin(config);
            
            // Serialize contents to string
            const rawContent = JSON.stringify(contents);
            
            try {
                // Call WASM synchronously (it returns a promise)
                const compressedContent = await engine.compress(rawContent, model);
                return {
                    modifiedContents: JSON.parse(compressedContent)
                };
            } catch (err) {
                console.error('[TokenMin] Compression failed, bypassing:', err);
                return { modifiedContents: contents }; // bypass
            }
        }
    }
};
`;
    fs.writeFileSync(pluginPath, wrapperCode.trim());
    console.log(`Plugin wrapper installed to: ${pluginPath}`);
    
    // Step 4: Register with config
    console.log('\n[4/4] Registering plugin in Gemini CLI config...');
    try {
        const configData = JSON.parse(fs.readFileSync(configPath, 'utf8'));
        
        if (!configData.plugins) {
            configData.plugins = [];
        }
        
        // Use relative path from config file to the plugin
        const relativePluginPath = './' + path.relative(configDir, pluginPath).replace(/\\\\/g, '/');
        
        if (!configData.plugins.includes(relativePluginPath)) {
            configData.plugins.push(relativePluginPath);
            fs.writeFileSync(configPath, JSON.stringify(configData, null, 2));
            console.log('Successfully registered TokenMin in gemini.config.json');
        } else {
            console.log('TokenMin is already registered in the configuration.');
        }
        
    } catch (e) {
        console.error('Failed to parse or update gemini.config.json:', e);
        console.log('Please manually add the plugin to your config:');
        console.log(`  "plugins": ["${pluginPath}"]`);
    }
    
    console.log('\n✨ TokenMin WASM installation complete!');
    rl.close();
}

main().catch(console.error);
