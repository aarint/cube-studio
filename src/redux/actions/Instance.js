export const INSTANCE_ADD = "INSTANCE_ADD";
export const INSTANCE_GET = "INSTANCE_GET";
export const INSTANCE_DELETE = "INSTANCE_DELETE";

export const INSTANCES_SAVED_GET = "INSTANCES_SAVED_GET";
export const INSTANCES_SAVED_GET_DONE = "INSTANCES_SAVED_GET_DONE";

export function addInstance(instance) {
    return {
        type: INSTANCE_ADD,
        instance
    }
}

export function getInstance(key) {
    return {
        type: INSTANCE_GET,
        key
    }
}

export function deleteInstance(instance) {
    return {
        type: INSTANCE_DELETE,
        instance
    }
}

export function getSavedInstances() {
    return {
        type: INSTANCES_SAVED_GET
    }
}

export function getSavedInstancesDone(instances) {
    return {
        type: INSTANCES_SAVED_GET_DONE,
        instances
    }
}