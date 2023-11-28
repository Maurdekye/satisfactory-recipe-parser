# Satisfactory Recipe Importer

This is a pair utility designed to work with the [Satisfactory Factory Planning Utility](https://github.com/Maurdekye/satisfactory_factory_planner). It parses the `Docs.json` file included with Satisfactory located with your installation of the game at `Steam\steamapps\common\Satisfactory\CommunityResources\Docs`, and renders the recipe information from it into the format usable by the planning utility.

The latest version of the planning utility, which will be 1.5.1 as of writing this, will include an up-to-date version of the recipe file generated from Update 8 of Satisfactory. If the game has not been updated since then, this program is unnecessary to run, as the recipes included with it should be up to date.

## Usage


```
satisfactory-recipe-parser.exe [OPTIONS] [DOCS_FILE]

Arguments:
  [DOCS_FILE]  Docs.json file to parse [default: Docs.json]

Options:
  -c, --config-file <CONFIG_FILE>  Config file [default: config.json]
  -o, --output <OUTPUT>            Output filename [default: recipes.json]
  -h, --help                       Print help
  -V, --version                    Print version
```

---

If the game updates and the program fails to run for the new game version, you may have to update `config.json` to include the additional native classes for the newly added items and machines, if applicable.  