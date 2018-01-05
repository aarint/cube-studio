import { setItem, getItem, removeItem } from './StorageUtil';

let savedInstances = [], connectedInstances = [], activeInstance = {};

export function getActiveInstance() {
    return activeInstance.instance;
}

export function addConnectedInstance(config, instance) {
    try {
        connectedInstances.push(instance);
        activeInstance = { config, instance };
    } catch (ex) {
        console.log('Wrong with adding aninstance');
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
        return savedInstances;
    } catch (ex) {
        console.log("Wrong with getting all instances.");
        return null;
    }
}
