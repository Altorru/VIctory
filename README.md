> **This fork is for 8 GiB laptop cards, and says so in one place:** [LAPTOP-8GB.md](LAPTOP-8GB.md)
> (30 fps sustained on an RTX 5070 Laptop, the launcher, the frame CSV, and the four builds
> the plan is built on). Everything below is upstream's own README.

# victory-leafy

**This may not work for you. You may need to pull certain changes and combine
them with your own build. It is here as a public resource that might help
someone.**

The cool_fox build people asked for, with codec_d's work, is on the
[`cool-fox`](https://github.com/101Leafy/victory-leafy/tree/cool-fox) branch.
On our NVIDIA machine it drew a grey scene, so it may not work either.

## Credits

This build stands on other people's work. The history is kept, every commit
keeps its author, and commits taken from elsewhere name the commit they came from.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end, and
  the shader recompiler.
- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**:
  the base of this build. Recorded list calls translated on threads of their own,
  prepared and joined submissions, write relay and landing, buffer fills on the
  card, cheaper write catching, a texture copy queue, frame record and replay, and
  the measurements in [docs/findings.md](docs/findings.md).
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  avatarmanz started from, and the people whose pull requests it carries.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[JoshuuDev/victory-amd](https://github.com/JoshuuDev/victory-amd)** and
  **myokex/victory-amd**: the guard over every view of a page (with Spexivus,
  joroyt, and tightfall), and Joshua Odom's reads into title memory.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes, cube
  gradients, sparse textures, and the descriptor heap split of pull request 68.
- **[uiliannn/VItoriaBr](https://github.com/uiliannn/VItoriaBr)**: packed dot
  products, barriers for narrow-wave GPUs, and the 64 KiB constant buffer clamp.
- **[darkworldhm/victory-fork-darkworldhm](https://github.com/darkworldhm/victory-fork-darkworldhm)**:
  the texture copy and buffer-view pass ideas behind several heap changes here.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**: render
  targets that are not set again.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back that makes
  boats float.
- Open pull requests to VIctory that are merged here: wave64 programs on 32-lane
  GPUs, f64 lowering, XMA audio, HLE and XAsync fixes, and more. Their authors are
  in the list below.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.

### Contributors

Everyone with a commit in this branch, by the name on their commits.

[0xkn0wh3r3](https://github.com/0xkn0wh3r3),
[101Leafy](https://github.com/101Leafy),
[3cvil](https://github.com/3cvil),
[9hvn9yt2vk-netizen](https://github.com/9hvn9yt2vk-netizen),
[Abdellah K.](https://github.com/Abdellah-Khanani),
[Advait Palve](https://github.com/AdvaitPalve11),
[AgneyaK](https://github.com/AgneyaK),
[AlanQiang0418](https://github.com/AlanQiang0418),
[alessiodeia](https://github.com/alessiodeia),
[alexandruiulian201-bot](https://github.com/alexandruiulian201-bot),
[Alpacher](https://github.com/Alpacher),
[Amine El Mahouti](https://github.com/amine23012004),
[Ammar Zubair](https://github.com/a-to-z),
[Andrea Casale (Maledetto)](https://github.com/Makto94),
[andrewpozdnakov7-jpg](https://github.com/andrewpozdnakov7-jpg),
[Andrey Kalyachko](https://github.com/Asenje),
[AnoHunted](https://github.com/AnoHunted),
[artor410](https://github.com/artor410),
[AscilCH](https://github.com/AscilCH),
ashrafsdrop,
[avatarmanz](https://github.com/avatarmanz),
[AwkGrep256](https://github.com/AwkGrep256),
[Aws](https://github.com/AwsAli05),
[BaggyALPHA](https://github.com/BaggyALPHA),
[Bartosz](https://github.com/Moon0755),
[BastienRdz](https://github.com/BastienRodz),
[Belobug](https://github.com/Belobug),
[Bloxive](https://github.com/ZekkJJ),
[borosf](https://github.com/borosf),
[brainbudt](https://github.com/brainbudt),
[broccolize](https://github.com/broccolize),
[Bullet18x](https://github.com/Bullet18x),
[bvtterbroot](https://github.com/bvtterbroot),
[ByFxbian](https://github.com/ByFxbian),
[Calvix2k](https://github.com/Calvix2k),
[Carlos Sant'Anna](https://github.com/SantannaCarlos),
[Castyan](https://github.com/Castyan),
Christian Thackston,
[coderanonymus](https://github.com/coderanonymus),
[compaccount1-coder](https://github.com/compaccount1-coder),
[CuteBooty](https://github.com/CuteBooty),
[cwWwWwW](https://github.com/CwWwWwW),
[cxccxc](https://github.com/cxchhh),
[Dathinel](https://github.com/Dathinel),
[Dave](https://github.com/davesousa),
[David Palacios](https://github.com/dpalacios69),
[Davide](https://github.com/wildheathen),
[davutsimsekk](https://github.com/davutsimsekk),
delacor,
[desp](https://github.com/desplmfao),
[Diego](https://github.com/DiegoFR20),
[divyeshganta](https://github.com/divyeshganta),
[DocBlack89](https://github.com/DocBlack89),
[DOT HEX](https://github.com/X13X),
[DoubleD17](https://github.com/DoubleD17),
[Douglas Sposito](https://github.com/SpositoDouglas),
[Duff](https://github.com/Dufftwt),
[Dukestar69](https://github.com/Dukestar69),
[EatCodeSleep2](https://github.com/EatCodeSleep2),
[Eduardo Bertozzi](https://github.com/EduBertozzi),
[Enzo Souza Duarte](https://github.com/enzoduarte1),
[ESAT](https://github.com/ESAT586),
[Esteban Viera](https://github.com/Esteban-V),
[Exopus](https://github.com/Exopus-0x65f7ad0),
[Exorion](https://github.com/Exorion1er),
[feanorsun](https://github.com/feanorsun),
[Felix2180](https://github.com/Felix2180),
[fever308](https://github.com/fever308),
[fffluffy-dev](https://github.com/fffluffy-dev),
[Filip722](https://github.com/Filip722),
[finegorko](https://github.com/finegorko),
[FIocker](https://github.com/FIocker),
[fluffyflattershy-afk](https://github.com/fluffyflattershy-afk),
[flxppr](https://github.com/flxppr),
[FrancescoGelosoEterea](https://github.com/FrancescoGelosoEterea),
[fuzeofficial](https://github.com/fuzeofficlal),
[fxceless](https://github.com/fxceless),
[GaiuLulius](https://github.com/GaiuLulius),
[George](https://github.com/GAD15),
[GhostRuntime](https://github.com/imnotsharingmyname),
Giovanni,
[Giuseppe Rainone](https://github.com/giusepperainone),
[Grimkaaah](https://github.com/Grimkaaah),
[HailLucifer666](https://github.com/HailLucifer666),
[Henrique Bueno](https://github.com/HenriqueSBueno),
[heromike405-arch](https://github.com/heromike405-arch),
[HoLa-Lee](https://github.com/HoLa-Lee),
[hophaver](https://github.com/hophaver),
[Hugo VIAUD](https://github.com/Altorru),
[ifBars](https://github.com/ifBars),
[illuminatedfellow](https://github.com/illuminatedfellow),
[imjustprism](https://github.com/imjustprism),
[Impre-visible](https://github.com/Impre-visible),
[Indy2l](https://github.com/Indy2l),
[ismailchaka1](https://github.com/ismailchaka1),
[itsRevela](https://github.com/itsRevela),
[Jake](https://github.com/Jrjy3),
[jaschinski97-ops](https://github.com/jaschinski97-ops),
[Jeremy Goldberg](https://github.com/jeremygold02),
[Jesper-Andersson](https://github.com/Jesper-Andersson),
[JigsawBR](https://github.com/JigsawBR),
[julio](https://github.com/juliocesarshopify-maker),
[Juraj Šimičević](https://github.com/JurekBurek),
[kais7654](https://github.com/kais7654),
[kekonn](https://github.com/kekonn),
[kesku](https://github.com/kesku),
[KoLolX3](https://github.com/KoLolX3),
[kripotker](https://github.com/kripotker),
[lameloball2813-ui](https://github.com/lameloball2813-ui),
lars.schwarzhofner,
[LaunchDoesntCode](https://github.com/LaunchDoesntCode),
[Lewandox](https://github.com/Lewandox),
LF2000 Builder,
[lhnizdo](https://github.com/lhnizdo),
[lordon1a](https://github.com/lordon1a),
[Louis Faust](https://github.com/catminou4),
[Ludovic Tourniquet](https://github.com/lerapeurdu62280-debug),
[LUIS LARIOS](https://github.com/Luislariosss),
[Luiz Neves](https://github.com/LouisRiverstone),
[Luiz Otavio Mello de Campos](https://github.com/Luizbc2),
[lukkasremondi](https://github.com/lukkasremondi),
[Lyrnc](https://github.com/Lyrnc),
[MamaLeVrai](https://github.com/MamaLeVrai),
[Manuel Pérez](https://github.com/AndyLatimer),
[Maraco91](https://github.com/Maraco91),
marquezini,
Martin Hartfelder,
[Marvis](https://github.com/Marvisak),
[Matsvey Makushenka](https://github.com/makmatvey),
[Mauro Alvarez](https://github.com/mauroalvare),
[Max Reis](https://github.com/bymaxdev),
[meower](https://github.com/a55uka),
[mgunes1311-lab](https://github.com/mgunes1311-lab),
[milesah3ad](https://github.com/milesah3ad),
moe-miau,
[nathanbzrr](https://github.com/nathanbzrr),
[neonbyte1](https://github.com/neonbyte1),
[Nico Panu](https://github.com/npanu420),
[nino-exe](https://github.com/nino-exe),
[Nixitov](https://github.com/Nixitov),
[Noname0202](https://github.com/MartinIndra02),
[Nostyxx](https://github.com/Nostyxx),
[ntgata1-dotcom](https://github.com/ntgata1-dotcom),
[NuraadGit](https://github.com/NuraadGit),
[oddiums](https://github.com/oddiums),
[Omeya Sawant](https://github.com/omeyasawant),
[Paulo Junior](https://github.com/N7z),
[Pavel Kotek](https://github.com/kotekompotek),
[Pavel](https://github.com/Pavl3000),
Pedro Fortunato,
[Peppacodes](https://github.com/Peppacodes),
[pes7](https://github.com/pes7),
[pgm1207](https://github.com/pablogonz12),
[Pilote Production](https://github.com/PiloteCode),
[pr0vId3nce](https://github.com/pr0vId3nce),
[pwu2775-dev](https://github.com/pwu2775-dev),
[quaternion](https://github.com/Quat3rnion),
[r4.](https://github.com/r4tedx),
[Rafael Convaitis](https://github.com/rafaconvaitis),
[raffclar](https://github.com/raffclar),
[Rivaldo Freitas de Carvalho](https://github.com/rivaldodev),
[Robin Snyders](https://github.com/Sr-0w),
[ruvenesh](https://github.com/ruvenesh),
[RVZO6](https://github.com/RVZO6),
[ryuki](https://github.com/YukiRyuu),
[Saitken91](https://github.com/SAitken91),
[salchipapa969-star](https://github.com/salchipapa969-star),
[Sebastian Kondratowicz](https://github.com/Creaa),
[Serph91P](https://github.com/Serph91P),
[Shadow](https://github.com/UnkwnDev),
[sharan104](https://github.com/sharan104),
[shh](https://github.com/reeledMarek),
[simbaui4life](https://github.com/simbaui4life),
Skyfrye,
[slatefield](https://github.com/slatefield),
[SlopComp](https://github.com/SlopComp),
[soujook](https://github.com/DoktorTK),
[spacede](https://github.com/space-de),
[Taynan Piza](https://github.com/TaynanGT),
[tedin7](https://github.com/tedin7),
TeresaPC,
[Tex](https://github.com/Texbirds),
[TG0000](https://github.com/TG0000),
[Therecalloftheonion](https://github.com/Therecalloftheonion),
[Thomas Askharoun](https://github.com/askharoun),
[TinkerBell-90](https://github.com/TinkerBell-90),
[Tissma95](https://github.com/Tissma95),
[undeadhuman37](https://github.com/undeadhuman37),
[UwUCammie](https://github.com/UwUCammie),
[Vanthys](https://github.com/Vanthys),
[Vax](https://github.com/Vaximous),
[vicennpacheco](https://github.com/vicennpacheco),
[VIREX](https://github.com/VlREX),
[Vyacheslav Venglovsky](https://github.com/venglov),
[vyto-line](https://github.com/Vyto-Line),
warmooff-ops,
[Wesney](https://github.com/wesneyalcantara-lang),
[wrothgaril](https://github.com/wrothgaril),
[wwwwwwww3333333](https://github.com/wwwwwwww3333333),
[Yaekai](https://github.com/Yaekai),
[YeokiNoewChi](https://github.com/YeokiNoewChi),
[Yoshi-233](https://github.com/Yoshi-233),
[yyc160](https://github.com/yyc160),
[ZazaJr24](https://github.com/ZazaJr24),
[ZHANG DAWEI](https://github.com/kalijerry),
[Zhelezniak Artem](https://github.com/Artking175),
[zocram4cc](https://github.com/zocram4cc),
[给伊莉雅补魔](https://github.com/geiyiliyabumo).

**Official Stream: https://youtube.com/live/da-WFG3W9ZI?feature=share**

101Leafy's build of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.
This branch brings together the forks and open pull requests above, with fixes
for NVIDIA cards, Intel CPUs, and Windows 10 on top. It is early work.

**This repository contains no game files, code, or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Master build (2026-11-27)

This branch now holds the master build, the one the stream runs. It runs the
title as the Series S does, at 1280x720. There is no ray tracing, no DLSS, no
FSR, no upscaling of any kind, and no frame generation.

Known issues:

- The first run builds every shader and stutters for a while.
- Cutscene faces don't animate yet: heads and bodies move, mouths and brows stay still. It is being fixed.

## Where it stood on 2026-11-26

Tested on one more PC since the 7900 XTX measurements: Windows 10, Intel
i9-13900K, NVIDIA RTX 5080. Menus and videos run at 30 fps. The open world runs
at 4-6 fps there and parts of the picture are still wrong (black blocks and dark
water where draws are dropped). The first run after a new build translates every
shader again and stutters for several minutes.

Measurements on the RX 7900 XTX, and what is known to be wrong there, are in
[docs/findings.md](docs/findings.md).

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/101Leafy/victory-leafy
cd victory-leafy
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on. `-Borderless`
(`VICTORY_BORDERLESS=1`) covers the primary display with no border or title bar;
the picture is stretched to it and the render size stays.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches, and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

On top of what the forks above bring:

- **NVIDIA**: the wave64 pull requests merged together, and float colour exports
  held to what the console's colour block allows, with a NaN written as 0
  (`VICTORY_COLOR_FINITE`).
- **Intel CPUs without SSE4a**: the title's `extrq` and `insertq` jump to SSE2
  code after their first fault (`VICTORY_PATCH_SSE4A`).
- **Windows 10**: the Agility SDK's D3D12Core is loaded so shader model 6.6 works.
- **Descriptors**: a capped host heap keeps the title's lower 393,215 slots, with
  a faster slot search, slots settled once a frame, and a buffer-view pass that
  looks only at slots that changed.
- **Pipelines**: a pipeline library folder that can be shared
  (`VICTORY_LIBRARY_DIR`), a shader cache keyed by the translator version, the
  known-pipelines book read on its own thread, and programs over 32 KiB linked as
  mesh pipelines.
- **Memory**: shadow copies over 1 MiB kept in card memory on 16 GB cards, a pool
  of read-back buffers, one shared zero buffer, and textures hashed once a frame.
- **Switch 2 Pro Controller** over USB.
- **Per-frame timing CSV** (`VICTORY_FRAMES`) for finding where a frame goes.

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
