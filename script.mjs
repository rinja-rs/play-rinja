window.gen_saved_url = function (callback) {
    new Promise(async (resolve) => {
        try {
            const rust = document.querySelector("#rust").value;
            const tmpl = document.querySelector("#tmpl").value;
            const data = new TextEncoder("utf-8").encode(
                "v1" + "\0" + rust + "\0" + tmpl
            );
            const comprStrm = new Blob([data])
                .stream()
                .pipeThrough(new CompressionStream("deflate"));
            const comprArray = await new Response(comprStrm).arrayBuffer();
            const comprBytes = new Uint8Array(comprArray);
            const base64 = btoa(String.fromCharCode(...comprBytes));
            const urlsafe = base64.replace(
                /[+/=]/g,
                (c) => ({ "+": "-", "/": "_", "=": "." }[c])
            );
            const url = new URL("?saved=" + urlsafe, location);
            resolve(String(url));
        } catch (e) {
            console.error("could not calculate state", e);
        } finally {
            resolve();
        }
    }).then(callback);
};

window.read_saved_url = function (callback) {
    new Promise(async (resolve) => {
        try {
            const url = new URL(location);
            const urlsafe = url.searchParams.get("saved");
            url.search = "";
            history.replaceState(null, "", url);
            if (!urlsafe) {
                return;
            }

            const base64 = urlsafe.replace(
                /[-_.]/g,
                (c) => ({ "-": "+", _: "/", ".": "=" }[c])
            );
            const comprStream = await fetch(
                "data:application/octet-binary;base64," + base64
            );
            const plainStream = comprStream.body.pipeThrough(
                new DecompressionStream("deflate")
            );
            const data = await new Response(plainStream).text();
            const [version, rust, tmpl] = data.split("\0", 3);
            if (version != "v1") {
                return;
            }
            resolve([rust, tmpl]);
        } catch (e) {
            console.error("could not read state", e);
        } finally {
            resolve();
        }
    }).then(function (data) {
        if (data) {
            const [rust, tmpl] = data;
            callback(rust, tmpl);
        } else {
            callback();
        }
    });
};

window.save_clipboard = function (text) {
    new Promise(async (resolve) => {
        try {
            const clipboard = window.navigator.clipboard;
            if (clipboard) {
                await clipboard.writeText(text);
            } else {
                alert("Clipboard is not available.");
            }
        } catch (e) {
            console.error("could not store to clipboard", e);
        } finally {
            resolve();
        }
    });
};

window.toggle_element = function (event, elementId) {
    if (event.target && event.target.id === elementId) {
        document.getElementById(elementId).classList.toggle("display");
    }
};

window.handle_blur = function (event, elementId) {
    const parent = document.getElementById(elementId);
    if (
        !parent.contains(document.activeElement) &&
        !parent.contains(event.relatedTarget)
    ) {
        parent.classList.remove("display");
    }
};

window.reset_code = function (event, text) {
    const input =
        event.target.parentElement.parentElement.querySelector("textarea");
    input.value = text;
    input.dispatchEvent(new Event("input"));
};

const state = history.state || {};
const reload_counter = +state.reload_counter || 0;
if (reload_counter > 0) {
    console.warn("reload_counter: ", reload_counter);
}
state.reload_counter = 0;
history.replaceState(state, "");

window.panic_reload = function () {
    state.reload_counter = reload_counter + 1;
    history.replaceState(state, "");
    window.setTimeout(function () {
        if (reload_counter > 2) {
            console.warn("Hit a panic. Three times.");
        } else {
            console.warn("Hit a panic, reloading.");
            window.location.reload();
        }
    }, 0);
};
