# FontCraft en GitHub con GitHub Desktop

La carpeta `app` (dentro de *Documentos ▸ Craft Anything ▸ FontCraft*) ya es un repositorio de Git
publicado en GitHub. Estas instrucciones usan la app **GitHub Desktop**; sus botones aparecen aquí
con su nombre en inglés y su traducción.

**Qué se sube.** El código, los iconos, el README y los instaladores (`packaging` y el flujo
`.github/workflows/build.yml`). El archivo `.gitignore` deja fuera `target` (las compilaciones, de
varios gigas), las capturas de las pruebas y los archivos `.DS_Store` del Finder. Nada de fuera de
`app` se sube (los registros de Craft Anything tampoco).

## 1. Publicar cambios

Cuando FontCraft cambie (por ejemplo, después de una ronda en Craft Anything):

1. Abre GitHub Desktop: en **Changes** (cambios) verás los archivos modificados.
2. Abajo a la izquierda, escribe un resumen corto del cambio, por ejemplo `Instaladores universales`.
3. Pulsa **Commit to main** (guardar el cambio).
4. Pulsa **Push origin** (enviarlo a GitHub). Se compila de nuevo solo.

## 2. Las compilaciones automáticas

Cada vez que envías cambios, GitHub compila FontCraft con el flujo **Compilar** y crea **un
instalador universal por sistema**: cada uno lleva las dos versiones de su sistema e instala la del
procesador del equipo.

| Trabajo | Instalador | Para |
| --- | --- | --- |
| macOS (Apple Silicon e Intel) · .dmg | `FontCraft-<versión>-macOS.dmg` | todos los Mac con macOS 12 o posterior |
| Windows (x64 y ARM64) · instalador | `FontCraft-<versión>-Windows-Setup.exe` y `…-Windows-Portable.zip` | PC con Windows 10 (22H2) u 11 de 64 bits, Intel, AMD o ARM |
| Linux (x64 y ARM64) · .deb | `fontcraft_<versión>_all.deb` | Debian, Ubuntu y derivados, en PC o en ARM |
| Android (ARM64 y x64) · .apk | `FontCraft-<versión>-Android.apk` | teléfonos, tabletas, Chromebooks y emuladores |
| iPad y iPhone · .ipa | `FontCraft-<versión>-iPadOS.ipa` | iPad y iPhone con iPadOS 15 o posterior |

Cada trabajo también **prueba su instalador**: lo instala, comprueba que eligió el programa del
procesador del equipo y que FontCraft arranca y, en Windows y Linux, que se desinstala sin dejar
nada. El .deb se prueba además en un equipo ARM64 emulado, y el de Windows comprueba que no necesita
el «Visual C++ Redistributable».

1. En la página del repositorio, abre la pestaña **Actions**.
2. Pulsa la ejecución más reciente de **Compilar**. La primera tarda entre 30 y 50 minutos; las
   siguientes, menos, porque GitHub guarda lo ya compilado.
3. Al acabar, el apartado **Artifacts** del final tiene un archivo por sistema: **FontCraft-macOS**,
   **FontCraft-Windows**, **FontCraft-Linux**, **FontCraft-Android** y **FontCraft-iPadOS**. Cada
   uno es un .zip con el instalador dentro.
4. Para compilar sin enviar nada nuevo: **Actions ▸ Compilar ▸ Run workflow**. Después de corregir
   algo, lanza siempre una ejecución nueva: **Re-run jobs** repite la versión del proyecto que había
   cuando se lanzó la anterior, sin los arreglos enviados después.

Si algún sistema sale en rojo, copia el mensaje de error y pídeme que lo arregle.

**Si el repositorio es privado:** GitHub da 2.000 minutos de compilación gratis al mes; los minutos
en Mac cuentan diez veces y cada envío compila dos veces en Mac (macOS e iPad). Si envías otro
cambio mientras compila, la compilación anterior se cancela sola. En un repositorio **público** las
compilaciones son gratuitas.

## 3. Publicar una versión con los instaladores

Los **Artifacts** solo se descargan con tu cuenta y caducan. Para dejar los instaladores en la página
del repositorio (**Releases**), con su número de versión:

1. Cambia la versión en `Cargo.toml` (la línea `version = "0.1.0"`, por ejemplo a `0.2.0`) y
   publícalo: **Commit to main ▸ Push origin**.
2. En GitHub Desktop, abre la pestaña **History** (historial), haz clic derecho en ese cambio y elige
   **Create Tag…**. Escribe la versión con una `v` delante, por ejemplo `v0.2.0`, y pulsa
   **Create Tag**.
3. Pulsa **Push origin**: envía la etiqueta.

GitHub compila todo y, si todo sale bien, crea la versión **FontCraft 0.2.0** en **Releases** con
los instaladores y `SHA256SUMS.txt`.

## El paquete de Android y su firma

Android solo instala paquetes firmados. **Sin hacer nada**, cada compilación usa una clave de prueba:
el paquete se instala para probarlo, pero para actualizar hay que desinstalar la versión anterior.
**Para publicarlo en Google Play** hace falta una clave permanente; créala una vez y guárdala en los
secretos del repositorio:

1. En la página del repositorio: **Settings ▸ Secrets and variables ▸ Actions ▸ New repository
   secret**.
2. Crea `ANDROID_KEYSTORE_BASE64` con el archivo de la clave convertido a texto (en Terminal,
   `base64 -i mi-clave.keystore | pbcopy` lo copia; después pégalo).
3. Crea `ANDROID_KEYSTORE_PASSWORD` con su contraseña.

Guarda una copia de la clave fuera del ordenador: sin ella no podrás publicar actualizaciones.

## Problemas frecuentes

- **Windows ARM64: «no se encontró VCRUNTIME140.dll»** (le pasó a PressCraft): FontCraft lleva el
  runtime de C dentro (`.cargo/config.toml`) y la compilación comprueba que ningún programa lo pide.
- **Windows: «Falta Inno Setup 6.3 o posterior»** (le pasó a PressCraft): el script busca Inno Setup
  en cualquier carpeta «Inno Setup…», en el registro de Windows y en el PATH, lee su versión donde
  la anota su instalador (ISCC.exe dice 0.0.0.0) y, si falta, lo instala con winget, Chocolatey o
  desde las versiones de sus autores en GitHub (el enlace de jrsoftware.org daba 404).
- **Linux ARM64 «Waiting for a runner»**: los equipos ARM de GitHub no siempre están disponibles en
  repositorios privados. Por eso las dos versiones de Linux se compilan en el mismo equipo (la de
  ARM64 en un contenedor de Ubuntu 22.04 con las bibliotecas de ARM64) y la de ARM64 se prueba en un
  equipo emulado.
- **Linux: «python3.12-minimal:arm64 … Exec format error»**: ocurría con la imagen de `cross`, que
  ahora es Ubuntu 24.04 y, al instalar las bibliotecas de ARM64, traía también el Python de ARM64,
  que no se puede preparar en un PC. Ya no se usa `cross`: el contenedor propio
  (`packaging/linux/Dockerfile`) es Ubuntu 22.04, donde esas bibliotecas se instalan sin él.
- **«The configuration 'macos-…' is not supported»**: GitHub retira las versiones antiguas de macOS;
  el flujo pide siempre la vigente (`macos-latest`).
- **«Publicar la versión» falla con «already exists»**: ya hay una versión con esa etiqueta. Usa un
  número nuevo (apartado 3).
- **Archivos de más de 100 MB**: GitHub los rechaza. FontCraft no tiene ninguno; si aparece alguno,
  probablemente se coló algo de `target`.
