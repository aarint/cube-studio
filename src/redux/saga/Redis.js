export function* doingString() {}

export function* addString(action) {
    console.log('add string', action.key, action.value);
}

export function* getString(action) {
    console.log('get', action.key)
}

export function* deleteString() {}