#import <UIKit/UIKit.h>

// Implemented in Rust (crates/game/src/lib.rs). Never returns: winit drives UIApplicationMain.
extern void main_rs(void);

int main(int argc, char *argv[]) {
    @autoreleasepool {
        // The profile file is written relative to GAME_STORAGE_DIR; the bundle itself is read-only.
        NSString *docs = NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, YES).firstObject;
        setenv("GAME_STORAGE_DIR", docs.UTF8String, 0);
        // No shell env on a device: assets/dev.env (written by scripts/build-ios-device.sh) provides
        // NAKAMA_HOST etc. Variables already set (e.g. SIMCTL_CHILD_*) win.
        NSString *envPath = [[NSBundle mainBundle] pathForResource:@"dev" ofType:@"env" inDirectory:@"assets"];
        NSString *envText = envPath ? [NSString stringWithContentsOfFile:envPath encoding:NSUTF8StringEncoding error:nil] : nil;
        for (NSString *line in [envText componentsSeparatedByCharactersInSet:NSCharacterSet.newlineCharacterSet]) {
            NSRange eq = [line rangeOfString:@"="];
            if (eq.location == NSNotFound || [line hasPrefix:@"#"]) continue;
            NSString *key = [[line substringToIndex:eq.location] stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceCharacterSet];
            NSString *val = [[line substringFromIndex:eq.location + 1] stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceCharacterSet];
            if (key.length) setenv(key.UTF8String, val.UTF8String, 0);
        }
        main_rs();
    }
    return 0;
}
