# Eww-Tray
[StatusNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/)
implementation for [eww](https://github.com/elkowar/eww)

### ⚠️ Eww-tray is a work in progress, expect bugs until first release. ⚠️

## Installation

**Cargo**

```shell
git clone https://github.com/oknozor/eww-tray
cd eww-tray
cargo install --path .
```

## Configuration

`eww-tray` will look on start for a config file under `$XDG_CONFIG_DIR/eww-tray.yuck`. 

The config consists of a single [tera](https://tera.netlify.app/) template file which is supposed to be rendered as an 
[eww literal widget](https://elkowar.github.io/eww/configuration.html#dynamically-generated-widgets-with-literal)

**Example:**

First we need to define a tera template for our system tray. 

```
# $HOME/.config/eww-tray.yuck
{% for tray_icon in tray_icons %}
    (image :path "{{tray_icon.icon_path}}" :image-width 24 :image-height 24)
{% endfor %}
```

Values surrounded with `{{` `}}` will be rendered dynamically whenever a system tray app changes. 
See [template](#template-context) for more info.

Now let's run `eww-tray` to ensure our template renders correctly. 
If you have any running apps interacting with the system tray you should see some outputs.

```
$ eww-tray
 (image :path "/tmp/.org.chromium.Chromium.vhJVvF/Element1_14.png" :image-width 24 :image-height 24)
 (image :path "/tmp/.org.chromium.Chromium.vhJVvF/Element1_14.png" :image-width 24 :image-height 24) (image :path "/home/okno/.local/share/Steam/public/steam_tray_mono.png" :image-width 24 :image-height 24)
```

Now let's tell `eww` about our system tray:
```
# eww.yuck
(deflisten tray_icons :initial "" "eww-tray")

(defwidget tray []
  (literal :content "(box :class 'tray' ${tray_icons})")
)
```

Add this widget to one of your eww bars and reload, you should now see something like this: 

![screenshot](docs/screenshot.png)

## Template context

```js
"tray_icons": [
  {
    "icon_path": String
  }   
]
```
