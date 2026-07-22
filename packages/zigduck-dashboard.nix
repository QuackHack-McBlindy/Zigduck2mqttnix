{ 
  self,
  lib,
  pkgs,
  stdenv,
  python3,
  fetchFromGitHub,
  ...
} : let 

  pythonEnv = python3.withPackages (ps: [
    ps.beautifulsoup4

  ]);
in
stdenv.mkDerivation {
  name = "zigduck-dashboard";
  src = ./zigduck-dashboard/src;

  buildInputs = [
    pythonEnv
  ];
  

  propagatedBuildInputs = [ pythonEnv ];

  installPhase = ''
    mkdir -p $out/bin
    echo "#!${pythonEnv}/bin/python3" > $out/bin/zigduck-dashboard
    cat $src/webserver.py >> $out/bin/zigduck-dashboard
    chmod +x $out/bin/zigduck-dashboard
  '';

  meta = with lib; {
    description = "Home automation system written in Rust";
    license = licenses.mit;
    maintainers = [ "QuackHack-McBlindy" ];
    mainProgram = "zigduck-dashboard";
    
  };}
