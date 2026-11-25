# FontCraft

Gestor de fuentes para macOS, Windows, Linux, Android e iPadOS: explora, previsualiza, imprime y
activa fuentes, con bibliotecas (`.fcl`), carpetas, listas inteligentes, Google Fonts y activación
automática para las apps de diseño (Affinity, Adobe, Apple, CorelDRAW, las apps ArtCraft,
PressCraft y muchas más). Escrito en Rust (eframe/egui). Cómo funciona por dentro:
[MECHANICS.md](MECHANICS.md).

## Instaladores

Hay **un instalador por sistema, universal**: lleva dentro FontCraft compilado para los dos
procesadores de ese sistema y, al instalarse, usa el del equipo. No hay que elegir versión.

| Sistema | Instalador | Lleva | Cómo elige |
| --- | --- | --- | --- |
| macOS 12 o posterior | `FontCraft-<versión>-macOS.dmg` | Apple Silicon e Intel en una sola app | macOS abre la mitad de su procesador |
| Windows 10 (22H2) y 11, 64 bits | `FontCraft-<versión>-Windows-Setup.exe` | x64 y ARM64 | el instalador mira el procesador e instala solo ese |
| Windows sin instalar | `FontCraft-<versión>-Windows-Portable.zip` | x64 y ARM64 | `FontCraft.exe` en Intel y AMD, `FontCraft-ARM64.exe` en ARM |
| Linux (Debian, Ubuntu y derivados) | `fontcraft_<versión>_all.deb` | amd64 y arm64 | al instalarse deja solo el del procesador del equipo |
| Android 12 o posterior | `FontCraft-<versión>-Android.apk` | ARM64 y x64 | Android instala la parte de su procesador |
| iPad y iPhone (iPadOS 15 o posterior) | `FontCraft-<versión>-iPadOS.ipa` | ARM64 | — |

GitHub los crea en cada envío y prueba cada uno instalándolo (ver
[SUBIR-A-GITHUB.md](SUBIR-A-GITHUB.md)). Las versiones etiquetadas `v…` quedan en **Releases** con
sus `SHA256SUMS.txt`.

### Crearlos en un ordenador

Con el código descargado, en la carpeta del proyecto (el resultado queda en `target/dist`):

| Sistema | Qué hace falta | Orden |
| --- | --- | --- |
| macOS | Rust y las herramientas de Xcode (`xcode-select --install`) | `packaging/macos/build-dmg.sh` |
| Windows | Rust, Visual Studio Build Tools con «Desarrollo para el escritorio con C++» y el componente «Herramientas de compilación de MSVC para ARM64», e Inno Setup 6.3 o posterior (el script lo instala si falta) | `powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1` |
| Linux | Rust, `sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev`, y Docker o Podman para la mitad del otro procesador (la compila `cross`) | `packaging/linux/build-deb.sh` |
| Android | el SDK y el NDK de Android, Java 17 y `ANDROID_HOME` | `packaging/android/build-apk.sh` |
| iPad y iPhone | un Mac con Rust y Xcode | `packaging/ipados/build-ipa.sh` |

Cada script compila las dos versiones primero; con `--no-build` (`-NoBuild` en Windows) usa las ya
compiladas. Para comprobar un instalador: `packaging/macos/check-dmg.sh`,
`sudo packaging/linux/check-deb.sh` o `packaging\windows\check-installer.ps1` (lo instala, mira que
el programa sea el del procesador del equipo, que arranque, y lo desinstala).

Windows: los dos programas llevan dentro el runtime de C de Microsoft (`.cargo/config.toml`), así
que arrancan en los PC sin el «Visual C++ Redistributable» (muchos ARM64, que si no dicen «no se
encontró VCRUNTIME140.dll»); el script y la prueba lo comprueban. Los scripts de Windows se guardan
con marca BOM para que Windows PowerShell lea bien los acentos.

### Al abrirlo por primera vez

Los instaladores aún no van firmados con un certificado de Apple o de Microsoft, así que el sistema
avisa una vez:

- **macOS:** arrastra FontCraft a Aplicaciones y ábrelo. Si macOS dice que no puede comprobar el
  desarrollador, abre **Ajustes del Sistema ▸ Privacidad y seguridad** y pulsa **Abrir igualmente**.
  Los permisos de Accesibilidad y Automatización que le des se conservan al instalar una versión
  nueva. Con un certificado «Developer ID Application» en el llavero,
  `APPLE_SIGNING_IDENTITY="Developer ID Application: …" packaging/macos/build-dmg.sh` lo firma con él.
- **Windows:** si aparece «Windows protegió su PC», pulsa **Más información ▸ Ejecutar de todas
  formas**. Se instala solo para ti (sin administrador) o, si lo eliges, para todos los usuarios;
  asocia los `.fcl` con su icono y se desinstala desde Configuración. En español y otros 6 idiomas.
- **Linux:** `sudo apt install ./fontcraft_<versión>_all.deb`. FontCraft aparece en el menú de
  aplicaciones y como la orden `fontcraft`. Necesita glibc 2.35 o posterior (Ubuntu 22.04, Debian 12
  o más recientes).
- **Android:** el `.apk` va firmado con una clave de prueba salvo que el repositorio tenga la suya
  (ver SUBIR-A-GITHUB.md).
- **iPad y iPhone:** el `.ipa` se instala con AltStore, Sideloadly o TrollStore, o con Xcode o Apple
  Configurator y una cuenta de Apple.

Datos de FontCraft: macOS `~/Library/Application Support/FontCraft`, Windows `%APPDATA%\FontCraft`
(o `FontCraftData` junto a la copia portátil), Linux `~/.local/share/FontCraft`, Android la carpeta
privada de la app.

## Estructura

| Carpeta o archivo | Qué hay |
| --- | --- |
| `src` | el código de FontCraft (el mismo para todos los sistemas) |
| `assets/icons` | los iconos de la app y de las bibliotecas `.fcl` (dibujos en `source`; `cargo run -- --make-icons assets/icons` los vuelve a generar, `--make-android-icons android/res` los de Android) |
| `packaging` | los instaladores de cada sistema y sus pruebas |
| `android`, `ipados` | los paquetes de Android e iPadOS (compilan `src` como app móvil) |
| `.github/workflows/build.yml` | la compilación automática en GitHub |
| `Cross.toml` | las bibliotecas que `cross` añade para compilar Linux para el otro procesador |
| `MECHANICS.md`, `mechanics.json` | cada valor y comportamiento, con su origen |
