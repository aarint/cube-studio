export const INSTANCE_ADD = "INSTANCE_ADD";
export const INSTANCE_GET = "INSTANCE_GET";
export const INSTANCE_DELETE = "INSTANCE_DELETE";

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