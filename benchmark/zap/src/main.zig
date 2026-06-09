const std = @import("std");
const zap = @import("zap");

fn on_request(r: zap.Request) void {
    if (r.path) |path| {
        if (std.mem.eql(u8, path, "/json")) {
            r.sendJson("{\"message\": \"Hello, World!\"}") catch return;
            return;
        }
    }
    r.sendBody("Hello, World!") catch return;
}

pub fn main() !void {
    var listener = zap.HttpListener.init(.{
        .port = 3000,
        .on_request = on_request,
        .log = false,
    });
    try listener.listen();
    zap.start(.{
        .threads = 4,
        .workers = 1,
    });
}
