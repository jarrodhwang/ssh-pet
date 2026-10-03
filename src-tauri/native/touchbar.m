#import <AppKit/AppKit.h>

// Public AppKit API only. The system shows this bar when a Droplet window is active.
typedef void (*DropletCallback)(const char *);
static NSString * const kOpenIdentifier = @"com.jarrod.droplet.open";
static NSString * const kConnectionsIdentifier = @"com.jarrod.droplet.connections";

@interface DropletTouchBar : NSObject <NSTouchBarDelegate>
@property(nonatomic, assign) DropletCallback callback;
@property(nonatomic, strong) NSTouchBar *touchBar;
@property(nonatomic, copy) NSArray<NSDictionary<NSString *, id> *> *connections;
@property(nonatomic, strong) NSScrollView *connectionScrollView;
@end

@implementation DropletTouchBar
- (NSTouchBarItem *)touchBar:(NSTouchBar *)touchBar makeItemForIdentifier:(NSTouchBarItemIdentifier)identifier {
    NSCustomTouchBarItem *item = [[NSCustomTouchBarItem alloc] initWithIdentifier:identifier];
    if ([identifier isEqualToString:kOpenIdentifier]) {
        NSImage *image = [NSImage imageWithSystemSymbolName:@"drop.fill" accessibilityDescription:@"Open Droplet"];
        NSButton *button = [NSButton buttonWithImage:image target:self action:@selector(open:)];
        button.toolTip = @"Open Droplet";
        item.view = button;
        item.customizationLabel = @"Droplet";
        return item;
    }

    if ([identifier isEqualToString:kConnectionsIdentifier]) {
        NSScrollView *scrollView = [NSScrollView new];
        scrollView.drawsBackground = NO;
        scrollView.hasHorizontalScroller = NO;
        scrollView.hasVerticalScroller = NO;
        scrollView.horizontalScrollElasticity = NSScrollElasticityAllowed;
        scrollView.translatesAutoresizingMaskIntoConstraints = NO;
        [NSLayoutConstraint activateConstraints:@[
            [scrollView.widthAnchor constraintEqualToConstant:440],
            [scrollView.heightAnchor constraintEqualToConstant:32],
        ]];
        self.connectionScrollView = scrollView;
        [self updateConnectionList:self.connections ?: @[]];
        item.view = scrollView;
        item.customizationLabel = @"Connections";
        return item;
    }

    return nil;
}

- (void)connect:(NSButton *)sender {
    if (self.callback) self.callback(sender.identifier.UTF8String);
}
- (void)open:(id)sender {
    if (self.callback) self.callback(NULL);
}

- (void)updateConnectionList:(NSArray<NSDictionary<NSString *, id> *> *)connections {
    self.connections = connections;
    NSMutableArray<NSButton *> *buttons = [NSMutableArray arrayWithCapacity:connections.count];
    for (NSDictionary<NSString *, id> *connection in connections) {
        NSString *connectionID = connection[@"id"];
        NSString *name = connection[@"name"];
        if (![connectionID isKindOfClass:[NSString class]] || connectionID.length == 0 ||
            ![name isKindOfClass:[NSString class]]) continue;
        NSButton *button = [NSButton buttonWithTitle:name target:self action:@selector(connect:)];
        button.identifier = connectionID;
        button.enabled = [connection[@"enabled"] boolValue];
        button.toolTip = [NSString stringWithFormat:@"Connect to %@", name];
        BOOL favorite = [connection[@"favorite"] boolValue];
        button.accessibilityLabel = favorite
            ? [NSString stringWithFormat:@"%@, favorite connection", name]
            : name;
        button.bezelColor = favorite
            ? [NSColor colorWithRed:0.22 green:0.55 blue:0.83 alpha:1.0]
            : nil;
        button.contentTintColor = favorite ? NSColor.whiteColor : nil;
        [buttons addObject:button];
    }
    NSStackView *stack = [NSStackView stackViewWithViews:buttons];
    stack.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    stack.alignment = NSLayoutAttributeCenterY;
    stack.spacing = 6;
    NSSize contentSize = stack.fittingSize;
    contentSize.height = 32;
    [stack setFrameSize:contentSize];
    self.connectionScrollView.documentView = stack;
}
@end

static NSMutableArray<DropletTouchBar *> *controllers;

void droplet_install_touchbar(void *windowPointer, DropletCallback callback) {
    NSWindow *window = (__bridge NSWindow *)windowPointer;
    if (!controllers) controllers = [NSMutableArray array];
    DropletTouchBar *controller = [DropletTouchBar new];
    controller.callback = callback;
    controller.connections = @[];
    NSTouchBar *bar = [NSTouchBar new];
    bar.delegate = controller;
    bar.defaultItemIdentifiers = @[kOpenIdentifier, kConnectionsIdentifier, NSTouchBarItemIdentifierOtherItemsProxy];
    bar.principalItemIdentifier = kOpenIdentifier;
    controller.touchBar = bar;
    window.touchBar = bar;
    [controllers addObject:controller];
}

void droplet_update_touchbar(const char *connectionsJSON) {
    if (!connectionsJSON) return;
    NSString *json = [NSString stringWithUTF8String:connectionsJSON];
    NSData *data = [json dataUsingEncoding:NSUTF8StringEncoding];
    id decoded = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if (![decoded isKindOfClass:[NSArray class]]) return;
    NSArray *connections = decoded;
    for (DropletTouchBar *controller in controllers) {
        [controller updateConnectionList:connections];
    }
}
