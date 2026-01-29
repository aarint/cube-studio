// Store for active instances
let instances = {};

export function addConnectedInstance(config, client) {
    instances[config.ip + ':' + config.port] = {
        ...config,
        client
    };
}

export function getActiveInstance() {
    const activeKey = Object.keys(instances)[0];
    return instances[activeKey]?.client || null;
}

export function deleteInstance(ip, port) {
    const key = `${ip}:${port}`;
    delete instances[key];
}

export function getConnectedInstances() {
    return Object.values(instances);
}
