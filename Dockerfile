FROM fedora:41

# Install dependencies
# flatpak-builder: to build the app
# flatpak: to manage runtimes
# cargo: to vendor dependencies
# git: required by cargo vendor
# make: often required by build scripts
RUN dnf install -y flatpak flatpak-builder cargo git make && \
    dnf clean all

# Add Flathub remote
RUN flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install Runtime and SDK
# This step downloads large files, so it's good to keep it in the image build
RUN flatpak install -y flathub org.gnome.Platform//49 org.gnome.Sdk//49 org.freedesktop.Sdk.Extension.rust-stable//25.08

WORKDIR /app

# Copy the entrypoint script
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Copy the project files
COPY . .

# Define the entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
