{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {
  options.house.zigbee.coordinator = mkOption {
    type = types.nullOr (types.submodule {
      options = {
        vendorId = mkOption {
          type = types.str;
          description = ''
            USB vendor ID of the Zigbee coordinator, in hexadecimal.

            You can find this with `lsusb`. For example, if it reports
            `ID 10c3:ea61`, the vendor ID is `10c3`.

            This is used together with `productId` to identify the correct
            USB device, regardless of which /dev/ttyUSB* or /dev/ttyACM*
            device name Linux assigns to it.
          '';
        };
        productId = mkOption {
          type = types.str;
          description = ''
            USB product ID of the Zigbee coordinator, in hexadecimal.

            You can find this with `lsusb`. For example, if it reports
            `ID 10c3:ea61`, the product ID is `ea61`.

            Together with `vendorId`, this identifies the coordinator USB
            device.
          '';
        };
        symlink = mkOption {
          type = types.str;
          description = ''
            Name of the stable symlink to create under /dev.

            For example, setting this to "zigbee" creates /dev/zigbee,
            which can then be used as the serial device for Zigbee2MQTT.

            Using a symlink avoids depending on unstable device names such
            as /dev/ttyUSB0, which can change when USB devices are plugged
            in or removed.
          '';
        };
        adapter = mkOption {
          type = types.str;
          description = ''
            Zigbee2MQTT adapter type used to communicate with the coordinator.

            For Zigbee2MQTT 2.x and newer, this should normally be set to
            the adapter type expected by your coordinator. Common values
            include "zstack" for Texas Instruments coordinators.

            If you are unsure, leave the default value unchanged.
          '';
          default = "zstack";
        };
      };
    });
    default = {};
    description = "Serial port device mapping by USB IDs";

  };}
