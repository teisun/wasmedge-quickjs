/**
 * Node 兼容层：child_process 经 hostcall node/exec 由宿主执行命令。
 * require('child_process') / require('node:child_process') 解析到本模块。
 */

function hostExec(params) {
    if (typeof globalThis.__pi_host_call !== 'function') {
        throw new Error('child_process.exec not available: __pi_host_call not configured');
    }
    const req = JSON.stringify({ module: 'node', method: 'exec', params });
    const resp = globalThis.__pi_host_call(req);
    const out = JSON.parse(resp);
    if (out && out.ok === false) {
        const err = new Error(out.error || 'exec failed');
        throw err;
    }
    return out.data || {};
}

function exec(cmd, options, callback) {
    if (typeof options === 'function') {
        callback = options;
        options = {};
    }
    options = options || {};
    const cb = callback;
    try {
        const data = hostExec({ cmd, options });
        const stdout = (data.stdout != null) ? String(data.stdout) : '';
        const stderr = (data.stderr != null) ? String(data.stderr) : '';
        const code = data.code != null ? data.code : 0;
        if (cb) {
            if (code !== 0) {
                const err = new Error(stderr || 'Command failed');
                err.code = code;
                process.nextTick(() => cb(err, stdout, stderr));
            } else {
                process.nextTick(() => cb(null, stdout, stderr));
            }
        }
    } catch (err) {
        if (cb) process.nextTick(() => cb(err, '', ''));
        else throw err;
    }
}

function execSync(cmd, options) {
    options = options || {};
    const data = hostExec({ cmd, options });
    const stdout = (data.stdout != null) ? String(data.stdout) : '';
    const stderr = (data.stderr != null) ? String(data.stderr) : '';
    const code = data.code != null ? data.code : 0;
    if (options.encoding === 'buffer' || options.encoding === undefined) {
        return Buffer.from(stdout);
    }
    if (code !== 0) {
        const err = new Error(stderr || 'Command failed');
        err.code = code;
        err.stdout = stdout;
        err.stderr = stderr;
        throw err;
    }
    return stdout;
}

export default {
    exec,
    execSync,
};
export {
    exec,
    execSync,
};
