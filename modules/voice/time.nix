{ 
  config,
  lib,
  pkgs,
  ...
} : let
in {
  yo.scripts.time = {
    description = "Get current time and dates.";
    category = "Home Automation";
    logLevel = "INFO";
    code = ''
      export LC_TIME=en_US.UTF-8
      TIME=$(date "+%H . %M")
      DAY=$(date "+%A")
      DATE=$(date "+%d %B")
      WEEK=$(date +%V)
      echo "The time is $TIME. It is $DAY, $DATE. Week $WEEK" 
      yo say "The time is $TIME. It is $DAY, $DATE. Week $WEEK"
    '';
    voice = {
      priority = 2;
      sentences = [
        "(what|what's|what) time is it"
        "what time is it"
        "(what|what's|what) day is it"
        "what's the date"
        "what's the date"
      ];
    };
    
  };}
