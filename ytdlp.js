#!/usr/bin/env node

/**
 * YouTube 视频直链提取器
 * 基于 yt-dlp 的实现原理
 * 支持通过 ejs 命令行工具解密 sig 和 n 参数
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const http = require('http');
const zlib = require('zlib');
const { URL } = require('url');
const { execSync } = require('child_process');

// ==================== 配置 ====================
const CONFIG = {
    // VIDEO_ID: 'BnnbP7pCIvQ',
    VIDEO_ID: 'E2Rj2gQAyPA',
    // E2Rj2gQAyPA
    EJS_PATH: 'C:\\Users\\Admin\\.ei\\ejs.exe',
    EJS_RUNTIME: 'qjs',
    OUTPUT_DIR: './youtube_data',
    USER_AGENT: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    ANDROID_USER_AGENT: 'com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip',
    // 客户端配置
    INNERTUBE_API_KEY: 'AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8',
    INNERTUBE_CLIENT_NAME: 'ANDROID',
    INNERTUBE_CLIENT_VERSION: '20.10.38'
};

// 创建输出目录
if (!fs.existsSync(CONFIG.OUTPUT_DIR)) {
    fs.mkdirSync(CONFIG.OUTPUT_DIR, { recursive: true });
}

// ==================== 工具函数 ====================

/**
 * 保存文件
 */
function saveFile(filename, content) {
    const filepath = path.join(CONFIG.OUTPUT_DIR, filename);
    if (typeof content === 'object') {
        fs.writeFileSync(filepath, JSON.stringify(content, null, 2), 'utf8');
    } else {
        fs.writeFileSync(filepath, content, 'utf8');
    }
    console.log(`已保存: ${filepath}`);
    return filepath;
}

/**
 * 格式化文件大小
 */
function formatFileSize(bytes) {
    if (!bytes) return 'Unknown';
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = parseInt(bytes);
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024;
        unitIndex++;
    }
    return `${size.toFixed(2)} ${units[unitIndex]}`;
}

// ==================== HTTP 请求 ====================

/**
 * HTTP(S) 请求封装 - 支持 gzip 解压和重定向
 */
function httpRequest(url, options = {}) {
    return new Promise((resolve, reject) => {
        const urlObj = new URL(url);
        const protocol = urlObj.protocol === 'https:' ? https : http;

        const requestOptions = {
            method: options.method || 'GET',
            headers: {
                'User-Agent': options.userAgent || CONFIG.USER_AGENT,
                'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
                'Accept-Language': 'en-US,en;q=0.9',
                'Accept-Encoding': 'gzip, deflate',
                ...options.headers
            }
        };

        const req = protocol.request(url, requestOptions, (res) => {
            // 处理重定向
            if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
                let redirectUrl = res.headers.location;
                if (redirectUrl.startsWith('/')) {
                    redirectUrl = `${urlObj.protocol}//${urlObj.host}${redirectUrl}`;
                }
                return httpRequest(redirectUrl, options).then(resolve).catch(reject);
            }

            const chunks = [];
            const encoding = res.headers['content-encoding'];

            let stream = res;
            if (encoding === 'gzip') {
                stream = res.pipe(zlib.createGunzip());
            } else if (encoding === 'deflate') {
                stream = res.pipe(zlib.createInflate());
            }

            stream.on('data', chunk => chunks.push(chunk));
            stream.on('end', () => {
                const body = Buffer.concat(chunks).toString('utf8');
                resolve({ statusCode: res.statusCode, headers: res.headers, body });
            });
            stream.on('error', reject);
        });

        req.on('error', reject);
        if (options.body) req.write(options.body);
        req.end();
    });
}

// ==================== 步骤 1: 获取视频页面 ====================

async function fetchVideoPage(videoId) {
    console.log('\n[步骤 1] 获取视频页面...');
    const url = `https://www.youtube.com/watch?v=${videoId}`;
    console.log(`URL: ${url}`);

    const response = await httpRequest(url, {
        headers: {
            'Cookie': 'CONSENT=YES+cb; SOCS=CAI'
        }
    });

    if (response.statusCode !== 200) {
        throw new Error(`获取页面失败: HTTP ${response.statusCode}`);
    }

    saveFile('page.html', response.body);
    console.log(`页面大小: ${(response.body.length / 1024).toFixed(2)} KB`);

    return response.body;
}

// ==================== 步骤 2: 提取 ytcfg 配置 ====================

function extractYtcfg(html) {
    console.log('\n[步骤 2] 提取 ytcfg 配置...');

    const ytcfg = {};

    // 提取 ytcfg.set() 调用
    const setPattern = /ytcfg\.set\s*\(\s*(\{[\s\S]*?\})\s*\)\s*;/g;
    let match;
    while ((match = setPattern.exec(html)) !== null) {
        try {
            const config = JSON.parse(match[1]);
            Object.assign(ytcfg, config);
        } catch (e) {
            // 尝试修复 JSON
            try {
                const fixed = match[1].replace(/'/g, '"').replace(/(\w+):/g, '"$1":');
                const config = JSON.parse(fixed);
                Object.assign(ytcfg, config);
            } catch (e2) {}
        }
    }

    // 提取单独的配置项
    const patterns = {
        'INNERTUBE_API_KEY': /"INNERTUBE_API_KEY"\s*:\s*"([^"]+)"/,
        'INNERTUBE_CLIENT_NAME': /"INNERTUBE_CLIENT_NAME"\s*:\s*"([^"]+)"/,
        'INNERTUBE_CLIENT_VERSION': /"INNERTUBE_CLIENT_VERSION"\s*:\s*"([^"]+)"/,
        'PLAYER_JS_URL': /"PLAYER_JS_URL"\s*:\s*"([^"]+)"/,
        'VISITOR_DATA': /"VISITOR_DATA"\s*:\s*"([^"]+)"/
    };

    for (const [key, pattern] of Object.entries(patterns)) {
        if (!ytcfg[key]) {
            const m = html.match(pattern);
            if (m) ytcfg[key] = m[1];
        }
    }

    saveFile('ytcfg.json', ytcfg);
    console.log(`提取到 ${Object.keys(ytcfg).length} 个配置项`);
    if (ytcfg.VISITOR_DATA) {
        console.log(`VISITOR_DATA: ${ytcfg.VISITOR_DATA.substring(0, 30)}...`);
    }

    return ytcfg;
}

// ==================== 步骤 3: 提取 Player URL ====================

function extractPlayerUrl(html, ytcfg) {
    console.log('\n[步骤 3] 提取 Player URL...');

    // 从 ytcfg 提取
    if (ytcfg.PLAYER_JS_URL) {
        const url = 'https://www.youtube.com' + ytcfg.PLAYER_JS_URL;
        console.log(`从 ytcfg 提取: ${url}`);
        return url;
    }

    // 从 HTML 提取
    const patterns = [
        /"jsUrl"\s*:\s*"([^"]+)"/,
        /"PLAYER_JS_URL"\s*:\s*"([^"]+)"/,
        /\/s\/player\/([a-zA-Z0-9_-]+)\/[^"]+?base\.js/
    ];

    for (const pattern of patterns) {
        const match = html.match(pattern);
        if (match) {
            let url = match[1] || match[0];
            url = url.replace(/\\\//g, '/').replace(/\\u0026/g, '&');
            if (url.startsWith('/')) {
                url = 'https://www.youtube.com' + url;
            }
            console.log(`从 HTML 提取: ${url}`);
            return url;
        }
    }

    // 提取 player ID 并构建 URL
    const playerIdMatch = html.match(/\/s\/player\/([a-zA-Z0-9_-]+)\//);
    if (playerIdMatch) {
        const playerId = playerIdMatch[1];
        const url = `https://www.youtube.com/s/player/${playerId}/player_ias.vflset/en_US/base.js`;
        console.log(`从 player ID 构建: ${url}`);
        return url;
    }

    throw new Error('无法提取 Player URL');
}

// ==================== 步骤 4: 下载 Player JS ====================

async function downloadPlayerJs(playerUrl) {
    console.log('\n[步骤 4] 下载 Player JS...');
    console.log(`URL: ${playerUrl}`);

    const response = await httpRequest(playerUrl);
    if (response.statusCode !== 200) {
        throw new Error(`下载 Player JS 失败: HTTP ${response.statusCode}`);
    }

    const playerJs = response.body;
    const filepath = saveFile('base.js', playerJs);
    console.log(`Player JS 大小: ${(playerJs.length / 1024).toFixed(2)} KB`);

    // 提取 player ID
    const playerIdMatch = playerUrl.match(/\/s\/player\/([a-zA-Z0-9_-]+)\//);
    const playerId = playerIdMatch ? playerIdMatch[1] : 'unknown';
    console.log(`Player ID: ${playerId}`);

    return { playerJs, filepath, playerId };
}

// ==================== 步骤 5: 提取 Player Response ====================

function extractPlayerResponseFromHtml(html) {
    console.log('\n[步骤 5] 提取 Player Response...');

    const patterns = [
        /var\s+ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;(?:\s*var\s+|<\/script>)/s,
        /ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;/s,
        /window\["ytInitialPlayerResponse"\]\s*=\s*(\{.+?\})\s*;/s
    ];

    for (const pattern of patterns) {
        const match = html.match(pattern);
        if (match) {
            try {
                // 处理可能的截断问题
                let jsonStr = match[1];
                // 尝试找到正确的 JSON 结束位置
                let braceCount = 0;
                let endIndex = 0;
                for (let i = 0; i < jsonStr.length; i++) {
                    if (jsonStr[i] === '{') braceCount++;
                    else if (jsonStr[i] === '}') braceCount--;
                    if (braceCount === 0) {
                        endIndex = i + 1;
                        break;
                    }
                }
                if (endIndex > 0) {
                    jsonStr = jsonStr.substring(0, endIndex);
                }

                const playerResponse = JSON.parse(jsonStr);
                saveFile('player_response_html.json', playerResponse);
                console.log('从 HTML 提取成功');
                return playerResponse;
            } catch (e) {
                console.log(`解析失败: ${e.message}`);
            }
        }
    }

    console.log('从 HTML 提取失败');
    return null;
}

// ==================== 步骤 6: 通过 Android API 获取 Player Response ====================

async function fetchPlayerResponseFromApi(videoId, visitorData = null) {
    console.log('\n[步骤 6] 通过 Android API 获取 Player Response...');

    const apiUrl = `https://www.youtube.com/youtubei/v1/player?key=${CONFIG.INNERTUBE_API_KEY}&prettyPrint=false`;

    // 使用 android_sdkless 配置 - 关键是不包含 androidSdkVersion
    const requestBody = {
        videoId: videoId,
        context: {
            client: {
                clientName: CONFIG.INNERTUBE_CLIENT_NAME,
                clientVersion: CONFIG.INNERTUBE_CLIENT_VERSION,
                // 注意：不要包含 androidSdkVersion，这是 android_sdkless 的关键区别
                userAgent: CONFIG.ANDROID_USER_AGENT,
                osName: 'Android',
                osVersion: '11',
                hl: 'en',
                timeZone: 'UTC',
                utcOffsetMinutes: 0
            }
        },
        playbackContext: {
            contentPlaybackContext: {
                html5Preference: 'HTML5_PREF_WANTS'
            }
        },
        contentCheckOk: true,
        racyCheckOk: true
    };

    const headers = {
        'Content-Type': 'application/json',
        'X-YouTube-Client-Name': '3',  // ANDROID
        'X-YouTube-Client-Version': CONFIG.INNERTUBE_CLIENT_VERSION,
        'Origin': 'https://www.youtube.com'
    };

    // 如果有 visitorData，添加到请求头
    if (visitorData) {
        headers['X-Goog-Visitor-Id'] = visitorData;
    }

    console.log('请求体:', JSON.stringify(requestBody, null, 2).substring(0, 500) + '...');
    saveFile('api_request.json', requestBody);

    const response = await httpRequest(apiUrl, {
        method: 'POST',
        userAgent: CONFIG.ANDROID_USER_AGENT,
        headers: headers,
        body: JSON.stringify(requestBody)
    });

    if (response.statusCode !== 200) {
        throw new Error(`API 请求失败: HTTP ${response.statusCode}`);
    }

    const playerResponse = JSON.parse(response.body);
    saveFile('player_response_api.json', playerResponse);
    console.log('API 请求成功');

    return playerResponse;
}

// ==================== 步骤 7: 使用 EJS 解密 ====================

function callEjs(playerJsPath, type, value) {
    try {
        // 构建命令
        const cmd = `"${CONFIG.EJS_PATH}" --runtime ${CONFIG.EJS_RUNTIME} "${playerJsPath}" ${type}:${value}`;
        console.log(`EJS 命令: ${cmd}`);

        const output = execSync(cmd, {
            encoding: 'utf8',
            maxBuffer: 50 * 1024 * 1024,
            timeout: 30000
        });

        console.log(`EJS 输出: ${output.substring(0, 200)}...`);
        saveFile(`ejs_${type}_output.json`, output);

        const result = JSON.parse(output);
        if (result.type === 'result' && result.responses && result.responses[0]) {
            const data = result.responses[0].data;
            return data[value];
        }
    } catch (e) {
        console.error(`EJS 调用失败: ${e.message}`);
    }
    return null;
}

function decryptNParam(playerJsPath, nValue) {
    console.log(`\n解密 n 参数: ${nValue}`);
    return callEjs(playerJsPath, 'n', nValue);
}

function decryptSignature(playerJsPath, sigValue) {
    console.log(`\n解密签名: ${sigValue.substring(0, 30)}...`);
    return callEjs(playerJsPath, 'sig', sigValue);
}

// ==================== 步骤 8: 处理格式 URL ====================

function processFormatUrl(format, playerJsPath) {
    let url = format.url;

    // 处理 signatureCipher
    if (!url && format.signatureCipher) {
        console.log('处理 signatureCipher...');
        const params = new URLSearchParams(format.signatureCipher);
        url = params.get('url');
        const s = params.get('s');
        const sp = params.get('sp') || 'signature';

        if (s && playerJsPath) {
            const decryptedSig = decryptSignature(playerJsPath, s);
            if (decryptedSig) {
                url += `&${sp}=${encodeURIComponent(decryptedSig)}`;
                console.log('签名解密成功');
            } else {
                console.log('签名解密失败');
                return null;
            }
        }
    }

    if (!url) return null;

    // 处理 n 参数
    try {
        const urlObj = new URL(url);
        const n = urlObj.searchParams.get('n');

        if (n && playerJsPath) {
            const decryptedN = decryptNParam(playerJsPath, n);
            if (decryptedN) {
                urlObj.searchParams.set('n', decryptedN);
                url = urlObj.toString();
                console.log('n 参数解密成功');
            } else {
                console.log('n 参数解密失败，使用原始 URL');
            }
        }
    } catch (e) {
        console.error(`URL 处理错误: ${e.message}`);
    }

    return url;
}

// ==================== 步骤 9: 提取所有格式 ====================

function extractFormats(playerResponse, playerJsPath) {
    console.log('\n[步骤 9] 提取所有格式...');

    const streamingData = playerResponse.streamingData;
    if (!streamingData) {
        throw new Error('无法获取 streamingData');
    }

    const formats = [
        ...(streamingData.formats || []),
        ...(streamingData.adaptiveFormats || [])
    ];

    console.log(`找到 ${formats.length} 个格式`);

    const processedFormats = [];

    for (const format of formats) {
        const itag = format.itag;
        const mimeType = format.mimeType || '';
        const quality = format.qualityLabel || format.quality || '';

        console.log(`\n处理格式 ${itag}: ${mimeType.split(';')[0]} ${quality}`);

        const url = processFormatUrl(format, playerJsPath);

        if (url) {
            processedFormats.push({
                itag,
                mimeType,
                quality,
                qualityLabel: format.qualityLabel,
                width: format.width,
                height: format.height,
                fps: format.fps,
                bitrate: format.bitrate,
                averageBitrate: format.averageBitrate,
                contentLength: format.contentLength,
                filesizeStr: formatFileSize(format.contentLength),
                audioQuality: format.audioQuality,
                audioSampleRate: format.audioSampleRate,
                audioChannels: format.audioChannels,
                url
            });
        }
    }

    return processedFormats;
}

// ==================== 步骤 10: 下载文件 ====================

async function downloadFile(url, outputPath) {
    console.log(`\n下载文件到: ${outputPath}`);

    return new Promise((resolve, reject) => {
        const urlObj = new URL(url);
        const protocol = urlObj.protocol === 'https:' ? https : http;

        const file = fs.createWriteStream(outputPath);

        const request = protocol.get(url, {
            headers: {
                'User-Agent': CONFIG.ANDROID_USER_AGENT,
                'Range': 'bytes=0-'
            },
            rejectUnauthorized: false // 忽略 SSL 证书验证
        }, (response) => {
            // 处理重定向
            if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
                file.close();
                fs.unlinkSync(outputPath);
                return downloadFile(response.headers.location, outputPath).then(resolve).catch(reject);
            }

            const totalSize = parseInt(response.headers['content-length'], 10) || 0;
            let downloadedSize = 0;

            response.pipe(file);

            response.on('data', (chunk) => {
                downloadedSize += chunk.length;
                if (totalSize > 0) {
                    const progress = ((downloadedSize / totalSize) * 100).toFixed(2);
                    process.stdout.write(`\r下载进度: ${progress}% (${formatFileSize(downloadedSize)} / ${formatFileSize(totalSize)})`);
                } else {
                    process.stdout.write(`\r已下载: ${formatFileSize(downloadedSize)}`);
                }
            });

            file.on('finish', () => {
                file.close();
                console.log('\n下载完成!');
                resolve();
            });
        });

        request.on('error', (err) => {
            fs.unlink(outputPath, () => {});
            reject(err);
        });

        file.on('error', (err) => {
            fs.unlink(outputPath, () => {});
            reject(err);
        });
    });
}

// ==================== 主函数 ====================

async function main() {
    try {
        console.log('='.repeat(70));
        console.log('YouTube 视频直链提取器');
        console.log('='.repeat(70));
        console.log(`视频 ID: ${CONFIG.VIDEO_ID}`);
        console.log(`EJS 路径: ${CONFIG.EJS_PATH}`);
        console.log(`输出目录: ${CONFIG.OUTPUT_DIR}`);

        // 步骤 1: 获取视频页面
        const html = await fetchVideoPage(CONFIG.VIDEO_ID);

        // 步骤 2: 提取 ytcfg 配置
        const ytcfg = extractYtcfg(html);

        // 步骤 3: 提取 Player URL
        const playerUrl = extractPlayerUrl(html, ytcfg);

        // 步骤 4: 下载 Player JS
        const { filepath: playerJsPath, playerId } = await downloadPlayerJs(playerUrl);

        // 步骤 5: 从 HTML 提取 Player Response (仅用于参考)
        const htmlPlayerResponse = extractPlayerResponseFromHtml(html);

        // 步骤 6: 使用 Android API 获取 Player Response (获取直接 URL)
        // Android 客户端不需要 PO Token，可以获取直接的流媒体 URL
        console.log('\n使用 Android 客户端获取流媒体 URL...');
        const playerResponse = await fetchPlayerResponseFromApi(CONFIG.VIDEO_ID, ytcfg.VISITOR_DATA);

        // 检查视频状态
        if (playerResponse.playabilityStatus) {
            const status = playerResponse.playabilityStatus;
            console.log(`\n视频状态: ${status.status}`);
            if (status.status !== 'OK') {
                console.log(`原因: ${status.reason || '未知'}`);
            }
        }

        // 步骤 9: 提取所有格式
        const processedFormats = extractFormats(playerResponse, playerJsPath);

        // 保存所有直链
        saveFile('direct_links.json', processedFormats);

        // 输出直链列表
        console.log('\n' + '='.repeat(70));
        console.log('直链列表:');
        console.log('='.repeat(70));

        // 分类输出
        const videoFormats = processedFormats.filter(f => f.mimeType.includes('video/'));
        const audioFormats = processedFormats.filter(f => f.mimeType.includes('audio/'));

        console.log('\n--- 视频格式 ---');
        videoFormats.forEach((fmt, index) => {
            console.log(`\n[${index + 1}] itag: ${fmt.itag}`);
            console.log(`    类型: ${fmt.mimeType.split(';')[0]}`);
            console.log(`    质量: ${fmt.qualityLabel || fmt.quality}`);
            if (fmt.width && fmt.height) {
                console.log(`    分辨率: ${fmt.width}x${fmt.height}`);
            }
            if (fmt.fps) {
                console.log(`    帧率: ${fmt.fps}fps`);
            }
            console.log(`    大小: ${fmt.filesizeStr}`);
            console.log(`    URL: ${fmt.url.substring(0, 100)}...`);
        });

        console.log('\n--- 音频格式 ---');
        audioFormats.forEach((fmt, index) => {
            console.log(`\n[${index + 1}] itag: ${fmt.itag}`);
            console.log(`    类型: ${fmt.mimeType.split(';')[0]}`);
            console.log(`    质量: ${fmt.audioQuality}`);
            if (fmt.audioSampleRate) {
                console.log(`    采样率: ${fmt.audioSampleRate}Hz`);
            }
            console.log(`    大小: ${fmt.filesizeStr}`);
            console.log(`    URL: ${fmt.url.substring(0, 100)}...`);
        });

        console.log('\n' + '='.repeat(70));

        // 输出完整的音频直链 (itag 140)
        const audioFormat = audioFormats.find(f => f.itag === 140);
        if (audioFormat) {
            console.log('\n完整音频直链 (itag 140 - AAC 128kbps):');
            console.log(audioFormat.url);

            console.log('\n下载命令:');
            console.log(`curl -L -H "Range: bytes=0-" -H "User-Agent: ${CONFIG.USER_AGENT}" "${audioFormat.url}" > audio.m4a`);

            // 下载音频
            console.log('\n开始下载音频...');
            const audioFile = path.join(CONFIG.OUTPUT_DIR, 'audio.m4a');
            await downloadFile(audioFormat.url, audioFile);
        }

        // 输出最高质量视频直链
        const bestVideo = videoFormats.sort((a, b) => (b.height || 0) - (a.height || 0))[0];
        if (bestVideo) {
            console.log(`\n最高质量视频直链 (itag ${bestVideo.itag} - ${bestVideo.qualityLabel}):`)
            console.log(bestVideo.url);
        }

        console.log('\n' + '='.repeat(70));
        console.log('完成!');
        console.log('='.repeat(70));

    } catch (error) {
        console.error('\n错误:', error.message);
        console.error(error.stack);
        process.exit(1);
    }
}

// 运行
if (require.main === module) {
    main();
}

module.exports = {
    fetchVideoPage,
    extractYtcfg,
    extractPlayerUrl,
    downloadPlayerJs,
    extractPlayerResponseFromHtml,
    fetchPlayerResponseFromApi,
    decryptNParam,
    decryptSignature,
    processFormatUrl,
    extractFormats,
    downloadFile,
    CONFIG
};
