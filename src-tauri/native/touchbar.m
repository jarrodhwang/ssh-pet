#import <AppKit/AppKit.h>

// Public AppKit API only. The system shows this bar when a Droplet window is active.
typedef void (*DropletCallback)(int);

@interface DropletTouchBar : NSObject <NSTouchBarDelegate>
@property(nonatomic, assign) DropletCallback callback;
@property(nonatomic, strong) NSButton *connectButton;
@property(nonatomic, copy) NSString *connectionTitle;
@property(nonatomic, assign) BOOL connectionEnabled;
@end

@implementation DropletTouchBar
- (NSTouchBarItem *)touchBar:(NSTouchBar *)touchBar makeItemForIdentifier:(NSTouchBarItemIdentifier)identifier {
    NSCustomTouchBarItem *item = [[NSCustomTouchBarItem alloc] initWithIdentifier:identifier];
    if ([identifier isEqualToString:@"com.jarrod.droplet.connect"]) {
        self.connectButton = [NSButton buttonWithTitle:self.connectionTitle ?: @"Connect" target:self action:@selector(connect:)];
        self.connectButton.bezelColor = [NSColor colorWithRed:0.22 green:0.55 blue:0.83 alpha:1.0];
        self.connectButton.enabled = self.connectionEnabled;
        item.view = self.connectButton;
        item.customizationLabel = @"Connect to favorite";
    } else {
        NSImage *image = [NSImage imageWithSystemSymbolName:@"drop.fill" accessibilityDescription:@"Open Droplet"];
        NSButton *button = [NSButton buttonWithImage:image target:self action:@selector(open:)];
        button.toolTip = @"Open Droplet";
        item.view = button;
        item.customizationLabel = @"Droplet";
    }
    return item;
}
- (void)connect:(id)sender { if (self.callback) self.callback(1); }
- (void)open:(id)sender { if (self.callback) self.callback(0); }
@end

static NSMutableArray<DropletTouchBar *> *controllers;

void droplet_install_touchbar(void *windowPointer, DropletCallback callback) {
    NSWindow *window = (__bridge NSWindow *)windowPointer;
    if (!controllers) controllers = [NSMutableArray array];
    DropletTouchBar *controller = [DropletTouchBar new];
    controller.callback = callback;
    NSTouchBar *bar = [NSTouchBar new];
    bar.delegate = controller;
    bar.defaultItemIdentifiers = @[@"com.jarrod.droplet.open", @"com.jarrod.droplet.connect", NSTouchBarItemIdentifierOtherItemsProxy];
    bar.principalItemIdentifier = @"com.jarrod.droplet.connect";
    window.touchBar = bar;
    [controllers addObject:controller];
}

void droplet_update_touchbar(const char *title, bool enabled) {
    NSString *label = [NSString stringWithUTF8String:title];
    for (DropletTouchBar *controller in controllers) {
        controller.connectionTitle = label;
        controller.connectionEnabled = enabled;
        controller.connectButton.title = label;
        controller.connectButton.enabled = enabled;
    }
}
