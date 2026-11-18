// No application globals or UI dependencies. Revision strings preserve u64s.
function open(database) {
    return new Promise((resolve, reject) => {
        let blocked = false;
        const request = indexedDB.open(database, 1);
        request.onupgradeneeded = () => request.result.createObjectStore('worlds');
        request.onerror = () => reject(String(request.error));
        request.onblocked = () => { blocked = true; reject('IndexedDB upgrade blocked by another tab'); };
        request.onsuccess = () => { if (blocked) request.result.close(); else resolve(request.result); };
    });
}
export async function loadWorld(database, slot) {
    const db = await open(database);
    try {
        return await new Promise((resolve, reject) => {
            const tx = db.transaction('worlds', 'readonly');
            const request = tx.objectStore('worlds').get(slot);
            let value;
            request.onsuccess = () => { value = request.result; };
            tx.oncomplete = () => resolve(value ? value.bytes : null);
            tx.onerror = tx.onabort = () => reject(String(tx.error));
        });
    } finally { db.close(); }
}
export async function commitWorld(database, slot, expected, revision, identity, bytes) {
    // Copy Rust's borrowed WASM memory before the first await.
    const copy = new Uint8Array(bytes);
    const db = await open(database);
    try {
        await new Promise((resolve, reject) => {
            let conflict = false;
            const tx = db.transaction('worlds', 'readwrite', {durability: 'strict'});
            const store = tx.objectStore('worlds');
            const request = store.get(slot);
            request.onsuccess = () => {
                const current = request.result;
                if ((current ? current.revision : '') !== expected || (current && current.identity !== identity)) {
                    conflict = true;
                    tx.abort();
                    return;
                }
                store.put({revision, identity, bytes: copy}, slot);
            };
            tx.oncomplete = () => resolve();
            tx.onerror = tx.onabort = () => reject(conflict ? 'save-conflict' : String(tx.error));
        });
    } finally { db.close(); }
}
