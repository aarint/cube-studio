import { setItem, getItem, removeItem } from './StorageUtil';

let currentInstances = [], activeInstance;

export function getActiveInstance() {
    return activeInstance;
}

export function addNewInstance(key, instance) {
    try {
        currentInstances.push({ key: key, instance: instance });
        activeInstance = instance;
    } catch (ex) {
        console.log('Wrong with adding aninstance');
    }
}

export function getInstanceByKey(key) {
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

export function deleteInstanceByKey(key) {
    try {
        key && currentInstances && currentInstances.forEach((item, index) => {

        })
    } catch (ex) {
        console.log("Wrong with deleting an instance.");
    }
}

export function getAllInstances() {
    try {
        return currentInstances;
    } catch (ex) {
        console.log("Wrong with getting all instances.");
        return null;
    }
}

export function getConfig() { }