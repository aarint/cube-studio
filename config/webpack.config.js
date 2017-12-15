module.exports = {
    externals: [
        (function () {
            var IGNORES = [
                'electron'
            ];
            return function (context, request, callback) {
                if (IGNORES.indexOf(request) >= 0) {
                    return callback(null, "require('" + request + "')");
                }
                return callback();
            };
        })()
    ],
    entry: {
        renderer: 'renderer.js'
    },
    output: {
        path: BUILD_PATH,
        filename: '[name].bundle.js',
    }
}