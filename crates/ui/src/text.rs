//! The interface texts in German and English (chosen from the system language at start).

use crate::model::{Connection, Presence, Reason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    /// From a POSIX locale value (`de_DE.UTF-8` → German); English otherwise.
    pub fn from_locale(v: &str) -> Self {
        if v.to_ascii_lowercase().starts_with("de") {
            Lang::De
        } else {
            Lang::En
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            Lang::De => &DE,
            Lang::En => &EN,
        }
    }
}

macro_rules! texts {
    ($($name:ident: $de:expr, $en:expr;)*) => {
        /// All fixed texts of the interface.
        pub struct Texts { $(pub $name: &'static str,)* pub shortcut_lines: &'static [&'static str], lang: Lang }
        pub static DE: Texts = Texts { $($name: $de,)* shortcut_lines: SHORTCUTS_DE, lang: Lang::De };
        pub static EN: Texts = Texts { $($name: $en,)* shortcut_lines: SHORTCUTS_EN, lang: Lang::En };
    };
}

const SHORTCUTS_DE: &[&str] = &[
    "Strg+K  Suche",
    "Strg+N  Neue Unterhaltung",
    "Strg+1 … 4  Chats, Kontakte, Anrufe, Meetings",
    "Enter  Senden · Umschalt+Enter  Zeilenumbruch",
    "Esc  Dialog oder Suche schließen",
    "Tab / Pfeiltasten  Fokus bewegen",
];
const SHORTCUTS_EN: &[&str] = &[
    "Ctrl+K  Search",
    "Ctrl+N  New conversation",
    "Ctrl+1 … 4  Chats, contacts, calls, meetings",
    "Enter  Send · Shift+Enter  New line",
    "Esc  Close dialog or search",
    "Tab / arrow keys  Move focus",
];

texts! {
    tagline: "PROFESSIONELLE KOMMUNIKATION", "PROFESSIONAL COMMUNICATION";
    search_hint: "Personen, Chats und Kontakte suchen", "Search people, chats and contacts";
    search_shortcut: "Strg K", "Ctrl K";
    profile_menu: "Status und Abmelden", "Status and sign-out";
    my_status: "Mein Status", "My status";
    sign_out: "Abmelden", "Sign out";
    chats: "Chats", "Chats";
    contacts: "Kontakte", "Contacts";
    calls: "Anrufe", "Calls";
    meetings: "Meetings", "Meetings";
    settings: "Einstellungen", "Settings";
    mono_chats: "CH", "CH";
    mono_contacts: "KO", "CO";
    mono_calls: "AN", "CA";
    mono_meetings: "ME", "ME";
    mono_settings: "EI", "SE";
    hide_context: "Seitenleiste ausblenden", "Hide side panel";
    show_context: "Seitenleiste einblenden", "Show side panel";
    show_context_setting: "Rechte Seitenleiste anzeigen", "Show the right side panel";
    status_demo: "DEMO · Keine Verbindung zu einem echten Server", "DEMO · Not connected to a real server";
    status_connected: "Verbunden (TLS, angemeldet)", "Connected (TLS, signed in)";
    status_degraded: "Eingeschränkt", "Degraded";
    status_connecting: "Verbindung wird aufgebaut …", "Connecting …";
    status_disconnected: "Verbindung getrennt", "Disconnected";
    status_failed: "Anmeldung fehlgeschlagen", "Sign-in failed";
    status_signed_out: "Nicht angemeldet", "Not signed in";
    messages_title: "Nachrichten", "Messages";
    new_chat_tip: "Neue Unterhaltung (Strg+N)", "New conversation (Ctrl+N)";
    filter_all: "Alle", "All";
    filter_unread: "Ungelesen", "Unread";
    filter_groups: "Gruppen", "Groups";
    filter_rooms: "Räume", "Rooms";
    add: "Hinzufügen", "Add";
    appear_away: "Als abwesend anzeigen", "Appear away";
    meeting_link_hint: "Besprechungslink (https://…)", "Meeting link (https://…)";
    new_meeting_dots: "Neue Besprechung …", "New meeting …";
    new_meeting: "Neue Besprechung", "New meeting";
    change_meeting: "Besprechung ändern", "Change meeting";
    change_dots: "Ändern …", "Change …";
    cancel_meeting: "Absagen", "Cancel meeting";
    copy_link: "Link kopieren", "Copy link";
    meeting_subject: "Betreff", "Subject";
    meeting_lifetime: "Nutzbar für", "Usable for";
    meeting_access: "Wer muss nicht im Wartebereich warten?", "Who doesn't have to wait in the lobby?";
    access_invited: "Nur eingeladene Personen", "Only people I invite";
    access_organisation: "Personen aus meiner Organisation", "People from my organisation";
    access_anyone: "Jeder, auch anonyme Personen mit dem Link", "Anyone, also anonymous people with the link";
    lobby: "Im Wartebereich", "In the lobby";
    admit: "Zulassen", "Admit";
    deny: "Ablehnen", "Deny";
    admit_all: "Alle zulassen", "Admit all";
    muted_mark: "stumm", "muted";
    make_presenter: "Zum Referenten machen", "Make a presenter";
    make_attendee: "Zum Teilnehmer machen", "Make an attendee";
    mute_user: "Stummschalten", "Mute";
    unmute_user: "Stummschaltung aufheben", "Unmute";
    remove_from_meeting: "Aus der Besprechung entfernen", "Remove from the meeting";
    mute_audience: "Publikum stummschalten", "Mute the audience";
    unmute_audience: "Stummschaltung des Publikums aufheben", "Unmute the audience";
    lock_meeting: "Besprechung sperren", "Lock the meeting";
    unlock_meeting: "Besprechung entsperren", "Unlock the meeting";
    add_contact_dots: "Kontakt hinzufügen …", "Add a contact …";
    new_group_dots: "Neue Gruppe …", "New group …";
    new_group: "Neue Gruppe", "New group";
    rename_group: "Gruppe umbenennen", "Rename group";
    delete_group: "Gruppe löschen", "Delete group";
    delete_group_not_empty: "Nur eine leere Gruppe kann gelöscht werden.", "Only an empty group can be deleted.";
    move_to_group: "In Gruppe verschieben", "Move to group";
    remove_contact: "Aus Kontaktliste entfernen", "Remove from contacts";
    add_to_contacts: "Zu Kontakten hinzufügen", "Add to contacts";
    relationship: "Beziehung", "Relationship";
    rel_family: "Freunde und Familie", "Friends and family";
    rel_workgroup: "Arbeitsgruppe", "Workgroup";
    rel_colleagues: "Kollegen", "Colleagues";
    rel_external: "Externe Kontakte", "External contacts";
    rel_blocked: "Blockiert", "Blocked";
    rel_auto: "Automatisch (Vorgabe des Servers)", "Automatic (server's default)";
    relationships_unavailable: "Die Beziehungen bestehender Kontakte liest diese Oberfläche noch nicht; ändern lassen sie sich im Kontaktmenü.", "This interface does not read the relationships of existing contacts yet; change them in the contact menu.";
    no_new_subscribers: "Niemand Neues hat Sie hinzugefügt.", "Nobody new has added you.";
    added_you: "hat Sie zur Kontaktliste hinzugefügt", "added you to their contacts";
    distribution_group: "Verteilerliste", "Distribution list";
    dg_loading: "Mitglieder werden aus dem Adressbuch geholt …", "Getting the members from the address book …";
    dg_failed: "Mitglieder nicht verfügbar", "Members not available";
    dg_empty: "Keine Mitglieder.", "No members.";
    dg_partial: "Der Server zeigt nur einen Teil der Mitglieder.", "The server shows only part of the members.";
    calendar_title: "Kalender", "Calendar";
    calendar_empty: "Keine Termine in den nächsten Tagen.", "No appointments in the next days.";
    tomorrow: "Morgen", "Tomorrow";
    private_appointment: "Privater Termin", "Private appointment";
    busy_free: "Frei", "Free";
    busy_tentative: "Mit Vorbehalt", "Tentative";
    busy_busy: "Gebucht", "Busy";
    busy_oof: "Abwesend", "Out of office";
    now_running: "Läuft gerade", "In progress";
    oof_on: "Automatische Antworten sind an", "Automatic replies are on";
    oof_no_text: "Ohne Antworttext.", "Without a reply text.";
    about: "Über CommSCraft", "About CommSCraft";
    about_body: "Freier Linux-Client für Skype for Business Server, aus offenen Spezifikationen gebaut. Lizenz und Quellen: siehe Projektseite.", "Free Linux client for Skype for Business Server, built from open specifications. Licence and sources: see the project page.";
    add_contact: "Kontakt hinzufügen", "Add contact";
    add_contact_unavailable: "Kontakt hinzufügen – in dieser Oberfläche noch nicht", "Add contact – not in this interface yet";
    always_on_top: "Immer im Vordergrund", "Always on top";
    audio_device_settings: "Audiogerät einstellen", "Audio device settings";
    audio_device_short: "Audio", "Audio";
    change_status: "Status ändern", "Change status";
    close_alert: "Hinweis schließen", "Close notice";
    close_window: "Fenster schließen", "Close window";
    company: "Firma", "Company";
    contact_card: "Visitenkarte", "Contact card";
    contacts_groups: "Gruppen", "Groups";
    contacts_new: "Neu", "New";
    contacts_relationships: "Beziehungen", "Relationships";
    contacts_status: "Status", "Status";
    contacts_view_unavailable: "Diese Ansicht gibt es in dieser Oberfläche noch nicht.", "This view is not in this interface yet.";
    department: "Abteilung", "Department";
    dial_pad: "Tastenfeld", "Keypad";
    filter_calls: "Anrufe", "Calls";
    find_someone: "Person suchen", "Find a person";
    ignore: "Ignorieren", "Ignore";
    incoming_call: "ruft an", "is calling";
    incoming_video_call: "ruft mit Video an", "is calling with video";
    invites_to_meeting: "lädt Sie zu einer Besprechung ein", "invites you to a meeting";
    main_menu: "Menü", "Menu";
    menu_file: "Datei", "File";
    menu_help: "Hilfe", "Help";
    menu_tools: "Extras", "Tools";
    more_options: "Weitere Optionen", "More options";
    no_conversations_yet: "Hier erscheinen Ihre letzten Unterhaltungen.", "Your recent conversations appear here.";
    note_hint: "Was gibt es Neues?", "What's new?";
    note_label: "Persönliche Notiz", "Personal note";
    notifications: "Benachrichtigungen", "Notifications";
    ok: "OK", "OK";
    open: "Öffnen", "Open";
    options: "Optionen", "Options";
    options_title: "Optionen", "Options";
    opt_account: "Konto", "Account";
    opt_alerts: "Benachrichtigungen", "Alerts";
    opt_alerts_dnd: "Bei „Nicht stören“", "When Do Not Disturb";
    opt_alerts_general: "Allgemein", "General";
    opt_alerts_strangers: "Personen außerhalb der Kontaktliste", "People outside the contact list";
    opt_app_window: "Anwendungsfenster", "Application window";
    opt_audio: "Audiogerät", "Audio device";
    opt_audio_choose: "Gerät, Pegel und Testanruf wählen", "Choose device, levels and test call";
    opt_autostart: "Beim Anmelden am Computer starten", "Start when I log on to the computer";
    opt_away_group: "Abwesende, Offline- und unbekannte Kontakte gesondert gruppieren", "Group away, offline and unknown contacts separately";
    opt_block_strangers: "Einladungen von Unbekannten blockieren", "Block invitations from unknown people";
    opt_by_availability: "Nach Verfügbarkeit", "By availability";
    opt_by_name: "Nach Name", "By name";
    opt_camera: "Kamera (Gerätedatei)", "Camera (device file)";
    opt_camera_body: "Die Kamera wird erst geöffnet, wenn ein Videostrom ausgehandelt ist. Gilt ab dem nächsten Videoanruf.", "The camera is opened only once a video stream is negotiated. Applies from the next video call.";
    opt_contacts: "Kontaktliste", "Contact list";
    opt_contacts_display: "Kontakte anzeigen mit", "Show contacts with";
    opt_contacts_order: "Reihenfolge", "Order";
    opt_contacts_show: "Gruppen", "Groups";
    opt_conversation_window: "Gesprächsfenster", "Conversation windows";
    opt_dnd_sharing: "„Nicht stören“ während einer Freigabe", "Do Not Disturb while sharing";
    opt_emoticons: "Emoticons anzeigen", "Show emoticons";
    opt_exchange: "Exchange", "Exchange";
    opt_exchange_body: "Kalenderstatus, Abwesenheitsnotiz, Unterhaltungsverlauf und Voicemail aus Exchange bietet zurzeit das Kommandozeilenwerkzeug.", "Calendar state, out-of-office note, conversation history and voicemail from Exchange are offered by the command-line tool for now.";
    opt_favorites_group: "Gruppe „Favoriten“ zeigen", "Show the Favourites group";
    opt_files: "Dateispeicherung", "File saving";
    opt_general: "Allgemein", "General";
    opt_im: "Chat", "IM";
    opt_language: "Sprache", "Language";
    opt_location: "Standort", "Location";
    opt_logging: "Protokollierung", "Logging";
    opt_logging_body: "Ein SIP-Kurzprotokoll (nur Methode/Status und CSeq) gibt es mit der Umgebungsvariablen COMMSCRAFT_VERBOSE=1.", "A short SIP trace (method/status and CSeq only) is available with COMMSCRAFT_VERBOSE=1.";
    opt_meeting_audio: "Besprechungsaudio automatisch beitreten", "Join meeting audio automatically";
    opt_meeting_participants: "Beim Beitritt die Teilnehmerliste zeigen", "Show the participant list when joining";
    opt_meetings: "Besprechungen", "Meetings";
    opt_message_alerts: "Hinweis bei neuen Nachrichten in nicht geöffneten Unterhaltungen", "Notify about new messages in conversations that are not open";
    opt_one_line: "Nur Name (einzeilig)", "Name only (one line)";
    opt_personal: "Persönlich", "Personal";
    opt_picture: "Mein Bild", "My picture";
    opt_pictures: "Bilder", "Pictures";
    opt_play_sounds: "Töne abspielen (Klingeln, Hinweistöne)", "Play sounds (ringing, alerts)";
    opt_quiet_busy: "Bei „Beschäftigt“ nur das Nötigste", "Keep sounds to a minimum when Busy";
    opt_quiet_dnd: "Keine Hinweise bei neuen Nachrichten", "No notifications for new messages";
    opt_record: "Aufzeichnen", "Record";
    opt_recording: "Aufzeichnung", "Recording";
    opt_reopen_conversations: "Unterhaltungen beim Anmelden wieder öffnen", "Reopen conversations when signing in";
    opt_save_files: "Empfangene Dateien speichern unter", "Save received files to";
    opt_share_location: "Standort für andere Programme freigeben", "Share my location with other programs";
    opt_show_my_picture: "Mein Bild zeigen", "Show my picture";
    opt_show_pictures: "Bilder von Kontakten zeigen", "Show pictures of contacts";
    opt_sounds: "Töne", "Sounds";
    opt_sounds_body: "Klingeln, Hinweistöne und Wartemusik können zusätzlich mit COMMSCRAFT_RING, COMMSCRAFT_NOTIFY und COMMSCRAFT_HOLD_MUSIC durch eigene Dateien ersetzt werden.", "Ringing, alert sounds and music on hold can also be replaced by own files with COMMSCRAFT_RING, COMMSCRAFT_NOTIFY and COMMSCRAFT_HOLD_MUSIC.";
    opt_spelling: "Rechtschreibung beim Tippen prüfen", "Check spelling as I type";
    opt_status: "Status", "Status";
    opt_status_body: "Die Leerlaufzeit kommt vom Desktop (GNOME oder KDE); ohne sie bleibt der Status, wie Sie ihn wählen. 0 Minuten schaltet den Schritt ab.", "The idle time comes from the desktop (GNOME or KDE); without it the status stays as you choose it. 0 minutes turns the step off.";
    opt_inactive_after: "Als inaktiv anzeigen nach", "Show me as inactive after";
    opt_away_after: "Als abwesend anzeigen nach weiteren", "Show me as away after a further";
    opt_tabs: "Unterhaltungen als Reiter in einem Fenster", "Conversations as tabs in one window";
    opt_text_size: "Schriftgröße im Chat", "Chat text size";
    opt_two_lines: "Name und Status (zweizeilig)", "Name and status (two lines)";
    opt_video: "Videogerät", "Video device";
    phone_label: "Telefon", "Phone";
    poll_new: "Umfrage erstellen", "Create a poll";
    present: "Präsentieren", "Present";
    present_slides: "Folien präsentieren", "Present slides";
    quit: "Beenden", "Quit";
    reset_status: "Status automatisch", "Automatic status";
    send_im: "Chatnachricht senden", "Send a message";
    share_desktop: "Bildschirm freigeben", "Share my screen";
    share_window: "Fenster freigeben", "Share a window";
    status_away: "Abwesend", "Away";
    status_away_offline: "Abwesend und offline", "Away and offline";
    status_busy: "Beschäftigt", "Busy";
    status_offline: "Offline oder unbekannt", "Offline or unknown";
    status_online: "Verfügbar", "Available";
    tab_contacts: "Kontakte", "Contacts";
    tab_conversations: "Unterhaltungen", "Conversations";
    tab_meetings: "Besprechungen", "Meetings";
    version: "Version", "Version";
    video_device_settings: "Videogerät einstellen", "Video device settings";
    view_history: "Unterhaltungen anzeigen", "Show conversations";
    whiteboard_new: "Whiteboard öffnen", "Open a whiteboard";
    load_rooms: "Chatrooms laden", "Load chat rooms";
    load_rooms_tip: "Verbindet mit dem Persistent-Chat-Server, listet die eigenen Räume und tritt ihnen bei", "Connects to the persistent chat server, lists your rooms and joins them";
    no_rooms: "Noch keine Chatrooms geladen.", "No chat rooms loaded yet.";
    chat_room: "Chatroom", "Chat room";
    room_joining: "Beitritt läuft …", "Joining …";
    filter_online: "Online", "Online";
    filter_favorites: "Favoriten", "Favourites";
    filter_missed: "Verpasst", "Missed";
    favorites: "Favoriten", "Favourites";
    recent_conversations: "Letzte Unterhaltungen", "Recent conversations";
    no_conversations: "Noch keine Unterhaltungen. Mit + oder Strg+N eine beginnen.", "No conversations yet. Start one with + or Ctrl+N.";
    no_conversations_filter: "Keine Unterhaltung passt zum Filter.", "No conversation matches the filter.";
    typing_short: "schreibt …", "typing …";
    conversation_failed: "Unterhaltung nicht zustande gekommen", "Conversation could not be opened";
    conversation_ended: "Die Unterhaltung ist beendet.", "The conversation has ended.";
    conversation_closed_hint: "Die Unterhaltung ist beendet – neu beginnen über Kontakte oder +", "The conversation has ended – start again from contacts or +";
    opening: "Wird geöffnet …", "Opening …";
    you: "Sie", "You";
    yesterday: "Gestern", "Yesterday";
    today: "Heute", "Today";
    favorite: "Als Favorit merken", "Add to favourites";
    unfavorite: "Aus Favoriten entfernen", "Remove from favourites";
    end_conversation: "Unterhaltung beenden", "End conversation";
    select_conversation: "Keine Unterhaltung ausgewählt", "No conversation selected";
    select_conversation_hint: "Links eine Unterhaltung wählen oder über Kontakte einen Chat beginnen.", "Pick a conversation on the left or start a chat from contacts.";
    group_chat: "Gruppenchat", "Group chat";
    search_in_chat: "Im Chat suchen", "Search in chat";
    call: "Anrufen", "Call";
    sending: "Wird gesendet", "Sending";
    delivered: "Zugestellt", "Delivered";
    not_delivered: "Nicht zugestellt", "Not delivered";
    not_delivered_to: "Nicht zugestellt an", "Not delivered to";
    delivery_unknown: "Zustellung unbekannt", "Delivery unknown";
    demo_content_title: "Demo-Inhalt", "Demo content";
    demo_content: "Diese Nachrichten sind erfundene Beispieldaten. Nichts wird gesendet oder empfangen.", "These messages are made-up sample data. Nothing is sent or received.";
    no_messages: "Noch keine Nachrichten.", "No messages yet.";
    attach: "+ Datei", "+ File";
    formatting: "Formatierung", "Formatting";
    send: "Senden", "Send";
    send_tip: "Enter sendet, Umschalt+Enter fügt eine Zeile ein", "Enter sends, Shift+Enter adds a line";
    invite: "Einladen", "Invite";
    details: "Details", "Details";
    group_info: "Gruppeninformationen", "Group information";
    contact_info: "Kontaktinformationen", "Contact information";
    members_roles: "Mitglieder und Rollen", "Members and roles";
    no_roster: "Die Teilnehmerliste ist noch nicht eingetroffen.", "The participant list has not arrived yet.";
    presenter: "Referent", "Presenter";
    function: "Funktion", "Role";
    availability: "Verfügbarkeit", "Availability";
    files: "Dateien", "Files";
    history_title: "Verlauf", "History";
    history_body: "Nachrichten bleiben nur bis zum Abmelden in diesem Fenster; es gibt kein lokales Archiv und keine Volltextsuche.", "Messages stay in this window until sign-out only; there is no local archive and no full-text search.";
    delivery_title: "Zustellung", "Delivery";
    delivery_body: "„Zugestellt“ heißt: der Server oder die Gegenstelle hat die Nachricht angenommen. Gelesen ist damit nicht bestätigt.", "“Delivered” means the server or peer accepted the message. It does not confirm that it was read.";
    invite_title: "In die Gruppe einladen", "Invite to the group";
    transfer_title: "Anruf übertragen", "Transfer the call";
    new_chat_title: "Neue Unterhaltung", "New conversation";
    new_chat_body: "SIP-Adresse eingeben oder einen Kontakt wählen.", "Enter a SIP address or pick a contact.";
    address_hint: "name@firma.example", "name@company.example";
    cancel: "Abbrechen", "Cancel";
    start_chat: "Chat starten", "Start chat";
    start_group: "Als Gruppe starten", "Start as group";
    start_group_tip: "Eine Gruppenunterhaltung (Ad-hoc-Konferenz) öffnen und die Person einladen", "Open a group conversation (ad-hoc conference) and invite the person";
    contacts_sub: "Personen und Verfügbarkeit", "People and availability";
    other_contacts: "Weitere Kontakte", "Other contacts";
    no_contacts: "Die Kontaktliste ist leer.", "The contact list is empty.";
    contacts_offline: "Ohne Verbindung sind keine Kontakte verfügbar.", "Without a connection no contacts are available.";
    no_contacts_filter: "Kein Kontakt passt zum Filter.", "No contact matches the filter.";
    source_contact_list: "Kontaktliste", "Contact list";
    source_address_book: "Adressbuch", "Address book";
    source_directory: "Adressbuchsuche (Server)", "Address book search (server)";
    source_demo: "Demonstrationsdaten", "Demonstration data";
    select_contact: "Kein Kontakt ausgewählt", "No contact selected";
    select_contact_hint: "Links einen Kontakt wählen. Doppelklick startet einen Chat.", "Pick a contact on the left. A double click starts a chat.";
    audio_call: "Audioanruf", "Audio call";
    contact_overview: "Kontaktübersicht", "Contact overview";
    status: "Status", "Status";
    sip_address: "SIP-Adresse", "SIP address";
    phone: "Telefon", "Phone";
    origin: "Herkunft", "Source";
    presence_hint: "Präsenzmeldungen sind Momentaufnahmen. Ein grüner Status ist kein Zustell- oder Lesebeleg.", "Presence is a snapshot. A green status does not prove delivery or reading.";
    organisation: "Organisation", "Organisation";
    not_known: "Nicht bekannt", "Not known";
    reachability: "Erreichbarkeit", "Reachability";
    reachability_body: "Chat und Audioanruf laufen über die angemeldete Sitzung; ob die Person sie annimmt, zeigt erst die Antwort.", "Chat and audio calls go through the signed-in session; whether the person answers shows only in the reply.";
    availability_body: "Live-Präsenz nur, wenn eine gültige Servermeldung vorliegt.", "Live presence only when a valid server notification arrived.";
    note: "Hinweis", "Note";
    demo_person: "Demo-Person und Demo-Daten; keine echte Verbindung.", "Demo person and demo data; no real connection.";
    calls_sub: "Gespräche und Verlauf", "Calls and history";
    active_call: "Aktives Gespräch", "Active call";
    recent_calls: "Letzte Gespräche", "Recent calls";
    no_calls: "Noch keine Anrufe in dieser Sitzung.", "No calls in this session yet.";
    incoming: "Eingehend", "Incoming";
    outgoing: "Ausgehend", "Outgoing";
    missed: "Verpasst", "Missed";
    status_code: "Status", "status";
    call_back_tip: "Doppelklick ruft zurück", "Double click to call back";
    call_dialing: "Wird gewählt …", "Dialling …";
    call_ringing: "Es klingelt …", "Ringing …";
    call_incoming: "Eingehender Anruf", "Incoming call";
    call_on_hold: "Gehalten", "On hold";
    call_connected: "Audio verbunden", "Audio connected";
    call_connected_video: "Audio und Video verbunden", "Audio and video connected";
    call_connected_screen: "Freigabe verbunden", "Sharing connected";
    call_ended: "Beendet", "Ended";
    no_active_call: "Kein laufender Anruf", "No call in progress";
    no_active_call_hint: "Anrufe beginnen über Kontakte oder den Chat-Kopf.", "Start calls from contacts or the chat header.";
    demo_call: "Demo – kein echter Anruf, kein Ton", "Demo – no real call, no sound";
    answer: "Annehmen", "Answer";
    decline: "Ablehnen", "Decline";
    mono_mute: "MI", "MI";
    mono_hold: "HA", "HO";
    mono_video: "VI", "VI";
    mono_transfer: "ÜB", "TR";
    mute: "Stummschalten", "Mute";
    unmute: "Ton an", "Unmute";
    hold: "Halten", "Hold";
    resume: "Fortsetzen", "Resume";
    video: "Video", "Video";
    transfer: "Übertragen", "Transfer";
    hang_up: "Anruf beenden", "End call";
    stop_viewing: "Ansehen beenden", "Stop viewing";
    call_hint_title: "Hinweis zum Anrufstatus", "About the call status";
    call_hint: "„Verbunden“ erscheint erst, wenn die Sitzung die Antwort der Gegenstelle gemeldet hat; Wählen und Klingeln werden getrennt angezeigt.", "“Connected” appears only once the session reported the peer's answer; dialling and ringing are shown separately.";
    call_hint_demo: "Anrufstatus, Timer, Halten und Stummschalten sind hier Demo-Daten; es wird nichts gewählt.", "Call state, timer, hold and mute are demo data here; nothing is dialled.";
    call_details: "Gesprächsdetails", "Call details";
    call_kind: "Gesprächsart", "Call type";
    call_kind_audio: "Audio · 1:1", "Audio · 1:1";
    call_kind_conference: "Audio · Konferenz", "Audio · conference";
    call_kind_video: "Audio und Video · 1:1", "Audio and video · 1:1";
    call_kind_screen: "Bildschirmfreigabe ansehen", "Viewing a shared screen";
    answer_video: "Mit Video annehmen", "Answer with video";
    video_call: "Videoanruf", "Video call";
    screen_offered: "teilt einen Bildschirm", "shares a screen";
    view_screen: "Ansehen", "View";
    video_starting: "Video wird verbunden …", "Connecting video …";
    video_window: "Das Bild erscheint in einem eigenen, abgeschotteten Fenster.", "The picture appears in a separate, sandboxed window.";
    video_failed: "Video beendet", "Video ended";
    participants: "Teilnehmer", "Participants";
    connection_state: "Verbindungsstatus", "Connection state";
    audio_devices: "Audiogeräte", "Audio devices";
    audio_devices_body: "Mikrofon und Lautsprecher über ALSA bzw. PipeWire (Standardgerät des Systems).", "Microphone and speaker through ALSA or PipeWire (the system's default device).";
    privacy: "Datenschutz", "Privacy";
    privacy_body: "Signalisierung über TLS, Medien über SRTP. Das ist keine Ende-zu-Ende-Verschlüsselung.", "Signalling over TLS, media over SRTP. This is not end-to-end encryption.";
    meetings_sub: "Besprechungen und Einladungen", "Meetings and invitations";
    refresh: "Aktualisieren", "Refresh";
    invitations: "Einladungen", "Invitations";
    invited_by: "Eingeladen von", "Invited by";
    join: "Teilnehmen", "Join";
    my_meetings: "Meine Besprechungen", "My meetings";
    no_meetings: "Keine Besprechungen bekannt.", "No meetings known.";
    untitled_meeting: "Besprechung ohne Titel", "Untitled meeting";
    valid_until: "Gültig bis", "Valid until";
    demo_meeting: "Demo-Besprechung – keine echte Konferenz", "Demo meeting – no real conference";
    conference_id: "Konferenz-ID", "Conference ID";
    meetings_title: "Besprechungen", "Meetings";
    meetings_hint: "Eine Besprechung links wählen oder mit „Jetzt besprechen“ eine Gruppe öffnen.", "Pick a meeting on the left or open a group with “Meet now”.";
    meet_now: "Jetzt besprechen", "Meet now";
    untitled_conferences: "Konferenzen ohne Betreff (aus Gruppenunterhaltungen) werden nicht aufgeführt.", "conferences without a subject (from group conversations) are not listed.";
    shares_screen: "gibt frei", "is sharing";
    view_shared: "Anzeigen", "View";
    join_meeting_video: "Der Besprechung mit Video beitreten", "Join the meeting with video";
    join_meeting_audio: "Audio der Besprechung beitreten", "Join the meeting audio";
    meet_now_tip: "Öffnet eine Gruppenunterhaltung, in die Personen eingeladen werden können", "Opens a group conversation people can be invited to";
    meeting_content_title: "Inhalte in Besprechungen", "Meeting content";
    meeting_content_body: "Whiteboards, Folien und Umfragen einer Besprechung stehen in deren Gruppenunterhaltung unter Details → Besprechungsinhalte. Video und Bildschirmfreigaben innerhalb von Besprechungen bietet zurzeit nur das Kommandozeilenwerkzeug.", "Whiteboards, slides and polls of a meeting are in its group conversation under details → meeting content. Video and shared screens inside meetings are offered only by the command-line tool for now.";
    meeting_details: "Besprechungsdetails", "Meeting details";
    meeting_audio: "Audio", "Audio";
    meeting_audio_body: "Konferenzaudio läuft über die angemeldete Sitzung.", "Conference audio runs through the signed-in session.";
    meeting_sharing: "Freigaben", "Sharing";
    meeting_sharing_body: "Bildschirme, die jemand direkt mit Ihnen teilt, zeigt diese Oberfläche; Freigaben innerhalb einer Besprechung noch nicht.", "Screens someone shares with you directly are shown here; sharing inside a meeting not yet.";
    persistent_chat: "Persistent Chat", "Persistent chat";
    search_results: "Suche", "Search";
    search_scope: "Durchsucht Kontakte, Titel offener Unterhaltungen und – wenn verfügbar – das Adressbuch des Servers.", "Searches contacts, titles of open conversations and – when available – the server's address book.";
    conversations: "Unterhaltungen", "Conversations";
    nothing_found: "Nichts gefunden.", "Nothing found.";
    search_more_letters: "Ab drei Zeichen wird das Adressbuch durchsucht.", "The address book is searched from three characters on.";
    no_message_search: "Nachrichtentexte werden nicht durchsucht: es gibt kein Archiv und keinen Index.", "Message texts are not searched: there is no archive and no index.";
    display: "Darstellung", "Display";
    high_contrast: "Hoher Kontrast", "High contrast";
    scale: "Skalierung", "Scale";
    account: "Konto", "Account";
    shortcuts: "Tastenkürzel", "Keyboard shortcuts";
    capabilities_title: "Funktionen und ihr Stand", "Functions and their state";
    capabilities_body: "Was diese Oberfläche jetzt anbietet. „Vorschau“ ist nur Gestaltung mit Demo-Daten und hat keine Wirkung.", "What this interface offers now. “Preview” is design with demo data only and has no effect.";
    gate_available: "Verfügbar", "Available";
    gate_preview: "Vorschau", "Preview";
    gate_unavailable: "Nicht verfügbar", "Unavailable";
    cap_chat: "Einzelchat", "One-to-one chat";
    cap_group_chat: "Gruppenchat (Ad-hoc-Konferenz)", "Group chat (ad-hoc conference)";
    cap_presence: "Präsenz", "Presence";
    cap_directory: "Adressbuchsuche", "Address book search";
    cap_audio: "Audioanruf", "Audio call";
    cap_hold: "Halten", "Hold";
    cap_transfer: "Übertragen", "Transfer";
    cap_video: "Videoanruf", "Video call";
    cap_meetings: "Besprechungen", "Meetings";
    cap_files: "Dateiübertragung", "File transfer";
    cap_rich_text: "Formatierter Text", "Formatted text";
    cap_persistent_chat: "Persistent Chat (Räume)", "Persistent chat (rooms)";
    cap_meeting_content: "Besprechungsinhalte", "Meeting content";
    content_title: "Besprechungsinhalte", "Meeting content";
    content_show: "Inhalte anzeigen", "Show content";
    content_show_tip: "Verbindet mit dem Webkonferenz-Server dieser Besprechung: Whiteboards, Folien, Umfragen", "Connects to this meeting's web conferencing server: whiteboards, slides, polls";
    content_connecting: "Verbindung wird aufgebaut …", "Connecting …";
    content_none: "Keine Inhalte in dieser Besprechung.", "No content in this meeting.";
    content_ended: "Verbindung beendet", "Connection ended";
    content_presented: "wird präsentiert", "presented";
    content_presenter: "Präsentiert von", "Presented by";
    content_whiteboard: "Whiteboard", "Whiteboard";
    content_slides: "Folien", "Slides";
    content_poll: "Umfrage", "Poll";
    content_notes: "Notizen", "Notes";
    content_file: "Datei", "File";
    content_other: "Inhalt", "Content";
    content_annotations: "Anmerkungen", "annotations";
    content_annotation: "Anmerkung", "annotation";
    content_slide_count: "Folien", "slides";
    content_poll_open: "Abstimmung offen", "Voting open";
    content_poll_closed: "Abstimmung geschlossen", "Voting closed";
    content_your_vote: "Ihre Stimme", "Your vote";
    content_not_drawn: "Folien und Whiteboard-Striche werden hier noch nicht gezeichnet; angezeigt werden Titel, Stand und Anzahl.", "Slides and whiteboard ink are not drawn here yet; titles, position and counts are shown.";
    content_leave: "Inhalte schließen", "Close content";
    cap_history_search: "Suche im Verlauf", "History search";
    cap_read_receipts: "Lesebestätigung", "Read receipts";
    security_title: "Sicherheit", "Security";
    security_body: "Verbindungen nur mit gültigem TLS-Zertifikat, kein unverschlüsselter Rückfall. Gesichert heißt nicht Ende-zu-Ende-verschlüsselt. Chats werden nicht auf dem Datenträger gespeichert.", "Connections only with a valid TLS certificate, no unencrypted fallback. Secured does not mean end-to-end encrypted. Chats are not stored on disk.";
    preview_hint: "Vorschau mit Demo-Daten – ohne Wirkung", "Preview with demo data – no effect";
    sign_in_title: "Anmelden", "Sign in";
    sign_in_body: "Bei Skype for Business Server Ihrer Organisation.", "To your organisation's Skype for Business Server.";
    sign_in_address: "Anmeldeadresse", "Sign-in address";
    sign_in_user: "Benutzername (optional)", "User name (optional)";
    sign_in_user_hint: "leer: wie die Anmeldeadresse", "empty: as the sign-in address";
    sign_in_domain: "Windows-Domäne (optional)", "Windows domain (optional)";
    sign_in_domain_hint: "z. B. FIRMA", "e.g. COMPANY";
    sign_in_password: "Kennwort", "Password";
    sign_in_server: "Server (optional)", "Server (optional)";
    sign_in_server_hint: "host:port – leer: automatisch suchen", "host:port – empty: find automatically";
    sign_in: "Anmelden", "Sign in";
    sign_in_again: "Neu anmelden", "Sign in again";
    sign_in_tls_dsk: "Mit Benutzerzertifikat (TLS-DSK)", "With a user certificate (TLS-DSK)";
    sign_in_tls_dsk_tip: "Holt beim ersten Mal ein Zertifikat beim Zertifikatsdienst des Servers (mit dem Kennwort) und meldet sich danach damit an – ohne Kennwort, solange es gültig ist.", "Gets a certificate from the server's certificate service the first time (with the password) and signs in with it afterwards – without a password while it is valid.";
    trust_title: "Server außerhalb Ihrer Domäne", "Server outside your domain";
    trust_no: "Nicht vertrauen", "Do not trust";
    sign_in_kerberos: "Kerberos versuchen (Windows-Domäne)", "Try Kerberos (Windows domain)";
    sign_in_kerberos_tip: "Nutzt ein vorhandenes Kerberos-Ticket des Systems, sonst das Kennwort; NTLM bleibt der Rückfall.", "Uses the system's Kerberos ticket if there is one, else the password; NTLM stays the fallback.";
    signing_in: "Anmeldung läuft …", "Signing in …";
    start_demo: "Demo ansehen", "View demo";
    start_demo_tip: "Oberfläche mit erfundenen Daten, ohne Server", "The interface with made-up data, without a server";
    sign_in_remember: "Kennwort im Schlüsselbund merken", "Remember the password in the keyring";
    sign_in_remember_tip: "Legt das Kennwort nach erfolgreicher Anmeldung im Schlüsselbund des Systems ab (Secret Service). Löschen: Optionen → Persönlich.", "Keeps the password in the system's keyring (Secret Service) after a successful sign-in. Remove it: Options → Personal.";
    sign_in_password_stored: "Kennwort im Schlüsselbund gespeichert – das Feld darf leer bleiben.", "Password kept in the keyring – the field may stay empty.";
    opt_keyring: "Schlüsselbund", "Keyring";
    opt_forget_password: "Gespeichertes Kennwort löschen", "Remove the stored password";
    opt_no_password_stored: "Kein Kennwort im Schlüsselbund gespeichert.", "No password kept in the keyring.";
    sign_in_note: "Das Kennwort wird nur gespeichert, wenn Sie es merken lassen – im Schlüsselbund des Systems. Zertifikate werden immer geprüft.", "The password is stored only when you ask for it – in the system's keyring. Certificates are always checked.";
}

impl Texts {
    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn presence(&self, p: Presence) -> &'static str {
        let de = self.lang == Lang::De;
        match p {
            Presence::Available => {
                if de {
                    "Verfügbar"
                } else {
                    "Available"
                }
            }
            Presence::Busy => {
                if de {
                    "Beschäftigt"
                } else {
                    "Busy"
                }
            }
            Presence::InACall => {
                if de {
                    "Im Gespräch"
                } else {
                    "In a call"
                }
            }
            Presence::DoNotDisturb => {
                if de {
                    "Nicht stören"
                } else {
                    "Do not disturb"
                }
            }
            Presence::BeRightBack => {
                if de {
                    "Bin gleich zurück"
                } else {
                    "Be right back"
                }
            }
            Presence::Away => {
                if de {
                    "Abwesend"
                } else {
                    "Away"
                }
            }
            Presence::Inactive => {
                if de {
                    "Inaktiv"
                } else {
                    "Inactive"
                }
            }
            Presence::OffWork => {
                if de {
                    "Außer Haus"
                } else {
                    "Off work"
                }
            }
            Presence::Offline => "Offline",
            Presence::Unknown => {
                if de {
                    "Status unbekannt"
                } else {
                    "Status unknown"
                }
            }
        }
    }

    pub fn reason(&self, r: Reason) -> &'static str {
        let de = self.lang == Lang::De;
        match r {
            Reason::NotConnected => {
                if de {
                    "Nicht verfügbar: keine Verbindung zum Server."
                } else {
                    "Unavailable: not connected to the server."
                }
            }
            Reason::NotImplemented => {
                if de {
                    "Im aktuellen Stand nicht implementiert."
                } else {
                    "Not implemented at this stage."
                }
            }
            Reason::CliOnly => {
                if de {
                    "In dieser Oberfläche noch nicht verfügbar; das Kommandozeilenwerkzeug bietet es an."
                } else {
                    "Not in this interface yet; the command-line tool offers it."
                }
            }
            Reason::ServerDisabled => {
                if de {
                    "Vom Server für dieses Konto nicht freigegeben."
                } else {
                    "Not enabled by the server for this account."
                }
            }
            Reason::NoDevice => {
                if de {
                    "Kein passendes Gerät gefunden."
                } else {
                    "No suitable device found."
                }
            }
            Reason::DemoOnly => {
                if de {
                    "In der Demo nicht vorgeführt."
                } else {
                    "Not shown in the demo."
                }
            }
        }
    }

    pub fn connection_short(&self, c: &Connection, demo: bool) -> &'static str {
        let de = self.lang == Lang::De;
        if demo {
            return if de { "Offline-Demo" } else { "Offline demo" };
        }
        match c {
            Connection::Connected => {
                if de {
                    "Angemeldet"
                } else {
                    "Signed in"
                }
            }
            Connection::Degraded(_) => {
                if de {
                    "Eingeschränkt"
                } else {
                    "Degraded"
                }
            }
            Connection::Connecting => {
                if de {
                    "Verbinde …"
                } else {
                    "Connecting …"
                }
            }
            Connection::Disconnected(_) => {
                if de {
                    "Getrennt"
                } else {
                    "Disconnected"
                }
            }
            Connection::Failed(_) => {
                if de {
                    "Fehler"
                } else {
                    "Failed"
                }
            }
            Connection::SignedOut | Connection::Demo => {
                if de {
                    "Nicht angemeldet"
                } else {
                    "Signed out"
                }
            }
        }
    }

    pub fn call_direction(&self, d: crate::model::CallDirection) -> &'static str {
        use crate::model::CallDirection as D;
        match (self.lang(), d) {
            (Lang::De, D::Incoming) => "Eingehend",
            (Lang::De, D::Outgoing) => "Ausgehend",
            (Lang::De, D::Missed) => "Verpasst",
            (_, D::Incoming) => "Incoming",
            (_, D::Outgoing) => "Outgoing",
            (_, D::Missed) => "Missed",
        }
    }

    pub fn members_count(&self, n: usize) -> String {
        match (self.lang, n) {
            (Lang::De, 1) => "1 Mitglied".into(),
            (Lang::De, n) => format!("{n} Mitglieder"),
            (Lang::En, 1) => "1 member".into(),
            (Lang::En, n) => format!("{n} members"),
        }
    }

    pub fn more_members(&self, n: usize) -> String {
        match self.lang {
            Lang::De => format!("und {n} weitere"),
            Lang::En => format!("and {n} more"),
        }
    }

    pub fn unread_count(&self, n: usize) -> String {
        match self.lang {
            Lang::De => format!("{n} ungelesen"),
            Lang::En => format!("{n} unread"),
        }
    }

    pub fn is_typing(&self, who: &str) -> String {
        match self.lang {
            Lang::De => format!("{who} schreibt …"),
            Lang::En => format!("{who} is typing …"),
        }
    }

    pub fn trust_body(
        &self,
        host: &str,
        user_domain: &str,
        named_by: &str,
        domain: &str,
    ) -> String {
        match self.lang {
            Lang::De => format!("Der Server {host} gehört nicht zu Ihrer SIP-Domäne {user_domain}. Er wurde von {named_by} über eine geprüfte HTTPS-Verbindung genannt. Wenn Sie Servern in {domain} vertrauen, geht Ihre Anmeldung (und das Kennwort) dorthin; die Entscheidung wird gespeichert."),
            Lang::En => format!("The server {host} is not in your SIP domain {user_domain}. It was named by {named_by} over a verified HTTPS connection. If you trust servers in {domain}, your sign-in (and password) goes there; the decision is remembered."),
        }
    }

    pub fn trust_yes(&self, domain: &str) -> String {
        match self.lang {
            Lang::De => format!("{domain} vertrauen"),
            Lang::En => format!("Trust {domain}"),
        }
    }

    pub fn write_to(&self, who: &str) -> String {
        match self.lang {
            Lang::De => format!("Nachricht an {who} schreiben …"),
            Lang::En => format!("Write to {who} …"),
        }
    }
}
