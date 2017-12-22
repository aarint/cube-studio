export function setItem(key, value) {
    if (!key) {
        return;
    }

    try {
        localStorage.setItem(key, value);
    } catch (ex) {
        if(ex.name ==="QuotaExceededError"){
            return ex.msg;
        }

        return ex;
    }
}

export function getItem(key) {
    if (!key) {
        return;
    }

    localStorage.getItem(key);
}

export function removeItem(key) {
    if (!key) {
        return;
    }

    localStorage.removeItem(key);
}

export function clearStorage() {
    localStorage.clear();
}