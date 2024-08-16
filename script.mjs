window.rinja_update_hash = function (rust, tmpl) {
    const data = { rust, tmpl };
    new Promise(async (resolve) => {
        try {
            const compressedBlob = new Blob([JSON.stringify(data)])
                .stream()
                .pipeThrough(new CompressionStream("deflate"));
            const compressedArray = await new Response(
                compressedBlob
            ).arrayBuffer();
            const compressedString = String.fromCharCode(
                ...new Uint8Array(compressedArray)
            );
            const hash = btoa(compressedString).replace(
                /[+/=]/g,
                (c) => ({ "+": "-", "/": "_", "=": "." }[c])
            );
            history.replaceState(data, "", "#!1," + hash);
        } catch (e) {
            console.log("Could not update hash", e);
        } finally {
            resolve();
        }
    });
};

window.rinja_read_hash = function (callback) {
    new Promise(async (resolve) => {
        try {
            const m = /^#!1,(.*)$/.exec(location.hash);
            if (!m) {
                return;
            }

            const hash = m[1].replace(
                /[-_.]/g,
                (c) => ({ "-": "+", _: "/", ".": "=" }[c])
            );
            const compressedResp = await fetch(
                "data:application/octet-binary;base64," + hash
            );
            const plainStream = compressedResp.body.pipeThrough(
                new DecompressionStream("deflate")
            );
            const { rust, tmpl } = await new Response(plainStream).json();
            resolve([rust, tmpl]);
        } catch (e) {
            console.log("Could not read hash", e);
        } finally {
            resolve();
        }
    }).then(callback);
};
