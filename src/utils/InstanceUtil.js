import { setItem, getItem, removeItem } from './StorageUtil';
import { encryptToStorageString, decryptFromStorageString } from './CryptoUtil';

const SAVED_INSTANCES_KEY = 'cubeStudio.savedInstances';

let savedInstances = [], connectedInstances = [], activeInstance = {};

function normalizeConfig(config) {
    if (!config) return null;
    return {
        name: config.name || '',
        ip: config.ip,
        port: Number(config.port),
        type: config.type || 'Redis',
        password: config.password || '',
    };
}

function configKey(config) {
    return `${config.type || 'Redis'}|${config.ip}:${Number(config.port)}`;
}

function loadSavedInstancesFromStorage() {
    const raw = getItem(SAVED_INSTANCES_KEY);
    const parsed = decryptFromStorageString(raw);
    if (Array.isArray(parsed)) {
        return parsed.map(normalizeConfig).filter(Boolean);
    }
    return [];
}

function persistSavedInstances(nextInstances) {
    savedInstances = nextInstances;
    setItem(SAVED_INSTANCES_KEY, encryptToStorageString(nextInstances));
}

export function getActiveInstance() {
    return activeInstance.instance;
}

export function addConnectedInstance(config, instance) {
    try {
        activeInstance = { config, instance };
        connectedInstances.push(instance);
        // Save by default unless explicitly disabled (e.g. Save checkbox off)
        if (config && config.save !== false) {
            upsertSavedInstance(config);
        }
    } catch (ex) {
        console.log('Wrong with adding aninstance');
    }
}

export function upsertSavedInstance(config) {
    try {
        const normalized = normalizeConfig(config);
        if (!normalized || !normalized.ip || !normalized.port) return;

        const current = loadSavedInstancesFromStorage();
        const key = configKey(normalized);
        const next = [
            normalized,
            ...current.filter((c) => configKey(c) !== key),
        ];
        persistSavedInstances(next);
    } catch (ex) {
        console.log('Wrong with saving instance config');
    }
}

export function getSavedInstanceByKey(key) {
    try {
        if (currentInstances) {
            return null;
        }

        currentInstances.filter((item, index) => {
            if (item.key === key) {
                return item.instance
            }
        })
    } catch (ex) {
        console.log("Wrong with getting an instance.");
        return null;
    }
}

export function getSavedInstances() {
    try {
        const current = loadSavedInstancesFromStorage();
        savedInstances = current;
        return current;
    } catch (ex) {
        console.log("Wrong with getting all instances.");
        return null;
    }
}
