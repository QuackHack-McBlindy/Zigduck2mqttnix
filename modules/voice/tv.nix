{
  self,
  lib,
  config,
  ... 
} : let
  englishNumbers = [
    "zero" "one" "two" "three" "four" "five" "six" "seven" "eight" "nine" "ten"
    "eleven" "twelve" "thirteen" "fourteen" "fifteen" "sixteen" "seventeen" "eighteen" "nineteen" "twenty"
    "twenty-one" "twenty-two" "twenty-three" "twenty-four" "twenty-five" "twenty-six" "twenty-seven" "twenty-eight" "twenty-nine" "thirty"
    "thirty-one" "thirty-two" "thirty-three" "thirty-four" "thirty-five" "thirty-six" "thirty-seven" "thirty-eight" "thirty-nine" "forty"
    "forty-one" "forty-two" "forty-three" "forty-four" "forty-five" "forty-six" "forty-seven" "forty-eight" "forty-nine" "fifty"
    "fifty-one" "fifty-two" "fifty-three" "fifty-four" "fifty-five" "fifty-six" "fifty-seven" "fifty-eight" "fifty-nine" "sixty"
  ];

  englishNumber = n: builtins.elemAt englishNumbers (n - 1);

in {   
   
  yo.scripts.tv = {
    description = "Android TV Controller. Fuzzy search all media types and creates playlist and serves over webserver for casting.";
    category = "🎧 Media Management";
    parameters = [
      { 
        name = "typ";
        description = ''
          Specify the type of command or the media type to search for.
          Supported commands are: 
            on, off, up, down, call, favorites, star. 
          Media Types:
            tv, movie, livetv, podcast, music, song, musicvideo, jukebox (random music), othervideo, youtube.
          Device Naviagation:
            nav_up, nav_down, nav_left, nav_right, nav_select, nav_menu, nav_back
        '';
        default = "tv";
        optional = true;
        values = [ # list of allowed values
          "on" "off" "up" "down" "next" "prev" "call" "favorites" "star" "tv" "movie"
          "livetv" "podcast" "music" "song" "musicvideo" "jukebox" "othervideo" "youtube"
          "nav_up" "nav_down" "nav_left" "nav_right" "nav_select" "nav_menu" "nav_back" "channel_up" "channel_down" 
        ];
      }
      { name = "search"; type = "string"; description = "Media to search"; optional = true; }
      { name = "room"; description = "Room name of device to play on"; optional = true; }
      { name = "season"; type = "string"; description = "Specific season to play"; optional = true; }
      { name = "shuffle"; type = "bool"; description = "Shuffle Toggle, true or false"; default = true; }
    ];
    
    binary = self.inputs.zigduck2mqttnix.packages.x86_64-linux.tv + "/bin/tv";

    voice = {
        priority = 1; # 1-5
        sentences = [
          # season specific search
          "[jag] (spel|spela|kör|start|starta) [upp|igång] {typ} {search} (säsong|season) {season} i {room}"
          "jag vill se {typ} {search} (säsong|season) {season} i {room}" 
          "[jag] (spel|spela|kör|start|starta) [upp|igång] {typ} {search} (säsong|season) {season}"
          "jag vill se {typ} {search} (säsong|season) {season}"       
          # room specific device control
          "[jag] (spel|spela|kör|start|starta) [upp|igång] {typ} {search} i {room}"
          "jag vill se {typ} {search} i {room}"    
          "jag vill lyssna på {typ} i {room}"
          "jag vill höra {typ} {search} i {room}"
          "{typ} (volym|volymen|avsnitt|avsnittet|låt|låten|skiten) i {room}"          
          "tv {typ} i {room}"
          # default player
          "[jag] (spel|spela|kör|start|starta) [upp|igång] {typ} {search}"
          "jag vill se {typ} {search}"    
          "jag vill lyssna på [mina] {typ}"
          "jag vill höra [mina] {typ}"
          "{typ} (volym|volymen|avsnitt|avsnittet|låt|låten|skiten)"       
          "tv {typ}"
          # append to favorites playlist
          "spara i {typ}"
          "lägg till den här [låten] i {typ}"
          # find remote (Nvidia Shield remote only)
          "ring {typ}"
          "hitta {typ}"
        ]; # lists are in word > out word
        lists = {
          typ.values = [          
          # media 
            { "in" = "[serie|serien|tvserien|tv-serien]"; out = "tv"; }
            { "in" = "[pod|podd|podcost|poddan|podden|podcast]"; out = "podcast"; }
            { "in" = "[slump|slumpa|random|musik|mix|shuffle]"; out = "jukebox"; }
            { "in" = "[artist|artisten|band|bandet|grupp|gruppen]"; out = "music"; }
            { "in" = "[låt|låten|sång|sången|biten]"; out = "song"; }
            { "in" = "[film|filmen]"; out = "movie"; }
            { "in" = "[ljudbok|ljudboken]"; out = "audiobook"; }
            { "in" = "[video|videon]"; out = "othervideo"; }
            { "in" = "[musicvideo|musikvideo]"; out = "musicvideo"; }
            { "in" = "[kanal|kanalen|kannal]"; out = "livetv"; }
            { "in" = "[youtube|you-tube|you|yt|yotub|yotube|yotub|tuben|juden]"; out = "youtube"; }     
            { "in" = "[news|nyhet|nyheter|nyheterna|senaste nytt]"; out = "news"; }               
          # star currently playing track (add to favourites)             
            { "in" = "[spellista|spellistan|spel lista|spel listan]"; out = "favorites"; }
          # playback            
            { "in" = "[paus|pause|pausa|tyst|tysta|mute|stop]"; out = "pause"; }
            { "in" = "[play|fortsätt|okej]"; out = "play"; }
            { "in" = "[öj|höj|höjj|öka|hej]"; out = "up"; }
            { "in" = "[sänk|sänkt|ner|ned]"; out = "down"; }
            { "in" = "[näst|nästa|nästan|next|fram|framåt]"; out = "next"; }
            { "in" = "[förr|förra|föregående|backa|bakåt]"; out = "previous"; }
          # add to favourites playlist                        
            { "in" = "[spara|add|adda|addera|lägg till]"; out = "star"; }
            { "in" = "[favorit|favoriter|bästa]"; out = "star"; }
          # on/off           
            { "in" = "[av|stäng av]"; out = "off"; }            
            { "in" = "på"; out = "on"; }      
          # calls remote (Nvidia Shield remote only)                        
            { "in" = "[fjärren|fjärrkontroll|fjärrkontrollen]"; out = "call"; }               
          ]; # search can be anything            
          search.wildcard = true;
          room.values = let
            # generate room patterns from config.house.rooms (or fallback from devices)
            roomNames = if (builtins.hasAttr "rooms" config.house) then
              builtins.attrNames config.house.rooms
            else
              builtins.attrNames (lib.groupBy (d: d.room) (lib.attrValues zigbeeDevices));

            sanitizeRoom = str: lib.toLower (lib.replaceStrings [ " " "/" ] [ "" "_" ] str);
            englishRoomPatterns = room: [
              room                                   # original
              "the ${room}"                          # with article
              "${room}s"                             # plural
              "the ${room}s"                         # plural with article
              (sanitizeRoom room)                    # sanitized (if spaces)
              "the ${sanitizeRoom room}"
            ];
          in
            lib.forEach roomNames (room: {
              "in" = "[" + lib.concatStringsSep "|" (lib.unique (englishRoomPatterns room)) + "]";
              out = room;
            });
          season.values = map (pair: {
            "in" = builtins.concatStringsSep "|" pair;
            out  = builtins.head pair;
          }) nums;
        };
    };

  };}
